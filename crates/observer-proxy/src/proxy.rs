use ::walletrpc::*;
use chrono::Utc;
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};
use tokio::{
    fs::{File, OpenOptions},
    io::AsyncWriteExt,
    sync::{mpsc, Mutex},
};
use tokio_stream::wrappers::ReceiverStream;
use tonic::{transport::Channel, Request, Response, Status, Streaming};

struct LogFile {
    file: File,
    seq: u64,
}

#[derive(Clone)]
pub struct Proxy {
    upstream: CompactTxStreamerClient<Channel>,
    log: Arc<Mutex<LogFile>>,
    session: Arc<String>,
}

fn display_hash(hash: &[u8]) -> String {
    hex::encode(hash.iter().rev().copied().collect::<Vec<_>>())
}
fn block_args(block: &BlockId) -> Value {
    json!({"height": block.height, "hash": display_hash(&block.hash)})
}
fn range_args(range: &BlockRange) -> Value {
    json!({"start": range.start.as_ref().map(|b| b.height), "end": range.end.as_ref().map(|b| b.height),
        "start_hash": range.start.as_ref().map(|b| display_hash(&b.hash)),
        "end_hash": range.end.as_ref().map(|b| display_hash(&b.hash)), "pool_types": range.pool_types})
}
fn address_range_args(filter: &TransparentAddressBlockFilter) -> Value {
    json!({"address": filter.address, "range": filter.range.as_ref().map(range_args)})
}
fn utxo_args(arg: &GetAddressUtxosArg) -> Value {
    json!({"addresses": arg.addresses, "start_height": arg.start_height, "max_entries": arg.max_entries})
}

type ForwardedStream<T> = ReceiverStream<Result<T, Status>>;

fn forward_stream<T: Send + 'static>(
    response: Response<Streaming<T>>,
) -> Response<ForwardedStream<T>> {
    let (metadata, mut stream, extensions) = response.into_parts();
    let (sender, receiver) = mpsc::channel(1);
    tokio::spawn(async move {
        loop {
            let message = tokio::select! {
                _ = sender.closed() => return,
                message = stream.message() => message,
            };
            match message {
                Ok(Some(message)) => {
                    if sender.send(Ok(message)).await.is_err() {
                        return;
                    }
                }
                Ok(None) => {
                    match stream.trailers().await {
                        Ok(Some(trailers)) => {
                            // Tonic encodes a terminal Status as HTTP/2 trailers. Code::Ok
                            // keeps successful completion while preserving custom trailers.
                            let status = Status::with_metadata(tonic::Code::Ok, "", trailers);
                            let _ = sender.send(Err(status)).await;
                        }
                        Err(status) => {
                            let _ = sender.send(Err(status)).await;
                        }
                        Ok(None) => {}
                    }
                    return;
                }
                Err(status) => {
                    let _ = sender.send(Err(status)).await;
                    return;
                }
            }
        }
    });
    Response::from_parts(metadata, ReceiverStream::new(receiver), extensions)
}

impl Proxy {
    pub async fn new(
        upstream: CompactTxStreamerClient<Channel>,
        path: &Path,
        session: String,
    ) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            tokio::fs::create_dir_all(parent).await?;
        }
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .await?;
        Ok(Self {
            upstream,
            log: Arc::new(Mutex::new(LogFile { file, seq: 0 })),
            session: Arc::new(session),
        })
    }
    async fn record<T>(
        &self,
        method: &str,
        request: &Request<T>,
        args: Value,
    ) -> Result<(), Status> {
        self.record_value(method, args, format!("{:?}", request.metadata()))
            .await
    }
    async fn record_value(
        &self,
        method: &str,
        args: Value,
        metadata: String,
    ) -> Result<(), Status> {
        let mut log = self.log.lock().await;
        log.seq += 1;
        let mut line = serde_json::to_vec(&json!({"seq": log.seq, "ts": Utc::now().to_rfc3339(),
            "session": self.session.as_str(), "method": method, "args": args, "metadata": metadata}))
            .map_err(|e| Status::internal(format!("serialize observation: {e}")))?;
        line.push(b'\n');
        log.file
            .write_all(&line)
            .await
            .map_err(|e| Status::internal(format!("write observation: {e}")))?;
        log.file
            .flush()
            .await
            .map_err(|e| Status::internal(format!("flush observation: {e}")))?;
        Ok(())
    }
}

#[tonic::async_trait]
#[allow(deprecated)] // The proxy must still forward the pinned protocol's legacy RPCs.
impl CompactTxStreamer for Proxy {
    async fn get_latest_block(&self, req: Request<ChainSpec>) -> Result<Response<BlockId>, Status> {
        self.record("GetLatestBlock", &req, json!({})).await?;
        self.upstream.clone().get_latest_block(req).await
    }
    async fn get_block(&self, req: Request<BlockId>) -> Result<Response<CompactBlock>, Status> {
        self.record("GetBlock", &req, block_args(req.get_ref()))
            .await?;
        self.upstream.clone().get_block(req).await
    }
    async fn get_block_nullifiers(
        &self,
        req: Request<BlockId>,
    ) -> Result<Response<CompactBlock>, Status> {
        self.record("GetBlockNullifiers", &req, block_args(req.get_ref()))
            .await?;
        self.upstream.clone().get_block_nullifiers(req).await
    }
    type GetBlockRangeStream = ForwardedStream<CompactBlock>;
    async fn get_block_range(
        &self,
        req: Request<BlockRange>,
    ) -> Result<Response<Self::GetBlockRangeStream>, Status> {
        self.record("GetBlockRange", &req, range_args(req.get_ref()))
            .await?;
        self.upstream
            .clone()
            .get_block_range(req)
            .await
            .map(forward_stream)
    }
    type GetBlockRangeNullifiersStream = ForwardedStream<CompactBlock>;
    async fn get_block_range_nullifiers(
        &self,
        req: Request<BlockRange>,
    ) -> Result<Response<Self::GetBlockRangeNullifiersStream>, Status> {
        self.record("GetBlockRangeNullifiers", &req, range_args(req.get_ref()))
            .await?;
        self.upstream
            .clone()
            .get_block_range_nullifiers(req)
            .await
            .map(forward_stream)
    }
    async fn get_transaction(
        &self,
        req: Request<TxFilter>,
    ) -> Result<Response<RawTransaction>, Status> {
        let arg = req.get_ref();
        self.record(
            "GetTransaction",
            &req,
            json!({"txid": display_hash(&arg.hash), "index": arg.index,
            "block": arg.block.as_ref().map(block_args)}),
        )
        .await?;
        self.upstream.clone().get_transaction(req).await
    }
    async fn send_transaction(
        &self,
        req: Request<RawTransaction>,
    ) -> Result<Response<SendResponse>, Status> {
        let raw = req.get_ref();
        let parsed = crate::transparent::inspect(&raw.data);
        self.record("SendTransaction", &req, json!({"bytes": raw.data.len(), "height": raw.height,
            "raw_tx_hex": hex::encode(&raw.data),
            "has_transparent_component": parsed.as_ref().ok().map(|p| p.has_transparent_component),
            "transparent": parsed.as_ref().ok(), "parse_error": parsed.as_ref().err().map(ToString::to_string)})).await?;
        self.upstream.clone().send_transaction(req).await
    }
    type GetTaddressTxidsStream = ForwardedStream<RawTransaction>;
    async fn get_taddress_txids(
        &self,
        req: Request<TransparentAddressBlockFilter>,
    ) -> Result<Response<Self::GetTaddressTxidsStream>, Status> {
        self.record("GetTaddressTxids", &req, address_range_args(req.get_ref()))
            .await?;
        self.upstream
            .clone()
            .get_taddress_txids(req)
            .await
            .map(forward_stream)
    }
    type GetTaddressTransactionsStream = ForwardedStream<RawTransaction>;
    async fn get_taddress_transactions(
        &self,
        req: Request<TransparentAddressBlockFilter>,
    ) -> Result<Response<Self::GetTaddressTransactionsStream>, Status> {
        self.record(
            "GetTaddressTransactions",
            &req,
            address_range_args(req.get_ref()),
        )
        .await?;
        self.upstream
            .clone()
            .get_taddress_transactions(req)
            .await
            .map(forward_stream)
    }
    async fn get_taddress_balance(
        &self,
        req: Request<AddressList>,
    ) -> Result<Response<Balance>, Status> {
        self.record(
            "GetTaddressBalance",
            &req,
            json!({"addresses": req.get_ref().addresses}),
        )
        .await?;
        self.upstream.clone().get_taddress_balance(req).await
    }
    async fn get_taddress_balance_stream(
        &self,
        req: Request<Streaming<Address>>,
    ) -> Result<Response<Balance>, Status> {
        self.record("GetTaddressBalanceStream", &req, json!({"phase": "start"}))
            .await?;
        let (metadata, extensions, mut input) = req.into_parts();
        let (sender, receiver) = mpsc::channel(1);
        let outgoing = Request::from_parts(metadata, extensions, ReceiverStream::new(receiver));
        let pump = async {
            while let Some(address) = input.message().await? {
                self.record_value(
                    "GetTaddressBalanceStream",
                    json!({"phase": "message", "address": address.address}),
                    String::new(),
                )
                .await?;
                sender
                    .send(address)
                    .await
                    .map_err(|_| Status::cancelled("upstream closed address stream"))?;
            }
            drop(sender);
            Ok::<_, Status>(())
        };
        let mut client = self.upstream.clone();
        // A client stream error fails this RPC; it must never turn into a successful partial balance.
        let (response, ()) = tokio::try_join!(client.get_taddress_balance_stream(outgoing), pump)?;
        Ok(response)
    }
    type GetMempoolTxStream = ForwardedStream<CompactTx>;
    async fn get_mempool_tx(
        &self,
        req: Request<GetMempoolTxRequest>,
    ) -> Result<Response<Self::GetMempoolTxStream>, Status> {
        self.record("GetMempoolTx", &req, json!({"exclude_txid_suffixes": req.get_ref().exclude_txid_suffixes.iter().map(hex::encode).collect::<Vec<_>>(), "pool_types": req.get_ref().pool_types})).await?;
        self.upstream
            .clone()
            .get_mempool_tx(req)
            .await
            .map(forward_stream)
    }
    type GetMempoolStreamStream = ForwardedStream<RawTransaction>;
    async fn get_mempool_stream(
        &self,
        req: Request<Empty>,
    ) -> Result<Response<Self::GetMempoolStreamStream>, Status> {
        self.record("GetMempoolStream", &req, json!({})).await?;
        self.upstream
            .clone()
            .get_mempool_stream(req)
            .await
            .map(forward_stream)
    }
    async fn get_tree_state(&self, req: Request<BlockId>) -> Result<Response<TreeState>, Status> {
        self.record("GetTreeState", &req, block_args(req.get_ref()))
            .await?;
        self.upstream.clone().get_tree_state(req).await
    }
    async fn get_latest_tree_state(
        &self,
        req: Request<Empty>,
    ) -> Result<Response<TreeState>, Status> {
        self.record("GetLatestTreeState", &req, json!({})).await?;
        self.upstream.clone().get_latest_tree_state(req).await
    }
    type GetSubtreeRootsStream = ForwardedStream<SubtreeRoot>;
    async fn get_subtree_roots(
        &self,
        req: Request<GetSubtreeRootsArg>,
    ) -> Result<Response<Self::GetSubtreeRootsStream>, Status> {
        let arg = req.get_ref();
        self.record("GetSubtreeRoots", &req, json!({"start_index": arg.start_index, "shielded_protocol": arg.shielded_protocol, "max_entries": arg.max_entries})).await?;
        self.upstream
            .clone()
            .get_subtree_roots(req)
            .await
            .map(forward_stream)
    }
    async fn get_address_utxos(
        &self,
        req: Request<GetAddressUtxosArg>,
    ) -> Result<Response<GetAddressUtxosReplyList>, Status> {
        self.record("GetAddressUtxos", &req, utxo_args(req.get_ref()))
            .await?;
        self.upstream.clone().get_address_utxos(req).await
    }
    type GetAddressUtxosStreamStream = ForwardedStream<GetAddressUtxosReply>;
    async fn get_address_utxos_stream(
        &self,
        req: Request<GetAddressUtxosArg>,
    ) -> Result<Response<Self::GetAddressUtxosStreamStream>, Status> {
        self.record("GetAddressUtxosStream", &req, utxo_args(req.get_ref()))
            .await?;
        self.upstream
            .clone()
            .get_address_utxos_stream(req)
            .await
            .map(forward_stream)
    }
    async fn get_lightd_info(&self, req: Request<Empty>) -> Result<Response<LightdInfo>, Status> {
        self.record("GetLightdInfo", &req, json!({})).await?;
        self.upstream.clone().get_lightd_info(req).await
    }
    async fn ping(&self, req: Request<Duration>) -> Result<Response<PingResponse>, Status> {
        self.record(
            "Ping",
            &req,
            json!({"interval_us": req.get_ref().interval_us}),
        )
        .await?;
        self.upstream.clone().ping(req).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_hash_is_display_order() {
        assert_eq!(display_hash(&[1, 2, 255]), "ff0201");
    }
    #[tokio::test]
    async fn logging_is_ordered_and_does_not_overwrite_existing_evidence() {
        let path =
            std::env::temp_dir().join(format!("darkroom-proxy-{}.jsonl", std::process::id()));
        let _ = tokio::fs::remove_file(&path).await;
        let client =
            CompactTxStreamerClient::new(Channel::from_static("http://127.0.0.1:1").connect_lazy());
        let proxy = Proxy::new(client.clone(), &path, "test".into())
            .await
            .unwrap();
        let (a, b) = tokio::join!(
            proxy.record_value("One", json!({}), String::new()),
            proxy.record_value("Two", json!({}), String::new())
        );
        a.unwrap();
        b.unwrap();
        assert!(Proxy::new(client, &path, "test".into()).await.is_err());
        let text = tokio::fs::read_to_string(&path).await.unwrap();
        let lines: Vec<Value> = text
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0]["seq"], 1);
        assert_eq!(lines[1]["seq"], 2);
        drop(proxy);
        tokio::fs::remove_file(path).await.unwrap();
    }
}
