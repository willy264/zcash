// Exercise real tonic transports, including initial metadata, trailers, and stream errors.
use crate::proxy::Proxy;
use ::walletrpc::*;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::{
    metadata::MetadataMap,
    transport::{Channel, Server},
    Request, Response, Status, Streaming,
};

struct TestBackend;
type TestStream<T> = tokio_stream::Iter<std::vec::IntoIter<Result<T, Status>>>;
#[tonic::async_trait]
impl CompactTxStreamer for TestBackend {
    async fn get_latest_block(
        &self,
        _request: Request<ChainSpec>,
    ) -> Result<Response<BlockId>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    async fn get_block(
        &self,
        _request: Request<BlockId>,
    ) -> Result<Response<CompactBlock>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    async fn get_block_nullifiers(
        &self,
        _request: Request<BlockId>,
    ) -> Result<Response<CompactBlock>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    type GetBlockRangeStream = TestStream<CompactBlock>;
    async fn get_block_range(
        &self,
        request: Request<BlockRange>,
    ) -> Result<Response<Self::GetBlockRangeStream>, Status> {
        if request
            .metadata()
            .get("x-roundtrip")
            .and_then(|v| v.to_str().ok())
            != Some("present")
        {
            return Err(Status::invalid_argument("proxy dropped request metadata"));
        }
        let failure = request.get_ref().start.as_ref().unwrap().height == 2;
        let mut trailers = MetadataMap::new();
        trailers.insert("x-trailer", "kept".parse().unwrap());
        let status = Status::with_metadata(
            if failure {
                tonic::Code::Unavailable
            } else {
                tonic::Code::Ok
            },
            if failure { "backend ended" } else { "" },
            trailers,
        );
        let messages = vec![
            Ok(CompactBlock {
                height: 17,
                ..Default::default()
            }),
            Err(status),
        ];
        let mut response = Response::new(tokio_stream::iter(messages));
        response
            .metadata_mut()
            .insert("x-initial", "kept".parse().unwrap());
        Ok(response)
    }
    type GetBlockRangeNullifiersStream = TestStream<CompactBlock>;
    async fn get_block_range_nullifiers(
        &self,
        _request: Request<BlockRange>,
    ) -> Result<Response<Self::GetBlockRangeNullifiersStream>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    async fn get_transaction(
        &self,
        _request: Request<TxFilter>,
    ) -> Result<Response<RawTransaction>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    async fn send_transaction(
        &self,
        _request: Request<RawTransaction>,
    ) -> Result<Response<SendResponse>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    type GetTaddressTxidsStream = TestStream<RawTransaction>;
    async fn get_taddress_txids(
        &self,
        _request: Request<TransparentAddressBlockFilter>,
    ) -> Result<Response<Self::GetTaddressTxidsStream>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    type GetTaddressTransactionsStream = TestStream<RawTransaction>;
    async fn get_taddress_transactions(
        &self,
        _request: Request<TransparentAddressBlockFilter>,
    ) -> Result<Response<Self::GetTaddressTransactionsStream>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    async fn get_taddress_balance(
        &self,
        _request: Request<AddressList>,
    ) -> Result<Response<Balance>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    async fn get_taddress_balance_stream(
        &self,
        _request: Request<Streaming<Address>>,
    ) -> Result<Response<Balance>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    type GetMempoolTxStream = TestStream<CompactTx>;
    async fn get_mempool_tx(
        &self,
        _request: Request<GetMempoolTxRequest>,
    ) -> Result<Response<Self::GetMempoolTxStream>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    type GetMempoolStreamStream = TestStream<RawTransaction>;
    async fn get_mempool_stream(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::GetMempoolStreamStream>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    async fn get_tree_state(
        &self,
        _request: Request<BlockId>,
    ) -> Result<Response<TreeState>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    async fn get_latest_tree_state(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<TreeState>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    type GetSubtreeRootsStream = TestStream<SubtreeRoot>;
    async fn get_subtree_roots(
        &self,
        _request: Request<GetSubtreeRootsArg>,
    ) -> Result<Response<Self::GetSubtreeRootsStream>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    async fn get_address_utxos(
        &self,
        _request: Request<GetAddressUtxosArg>,
    ) -> Result<Response<GetAddressUtxosReplyList>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    type GetAddressUtxosStreamStream = TestStream<GetAddressUtxosReply>;
    async fn get_address_utxos_stream(
        &self,
        _request: Request<GetAddressUtxosArg>,
    ) -> Result<Response<Self::GetAddressUtxosStreamStream>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    async fn get_lightd_info(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<LightdInfo>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
    async fn ping(&self, _request: Request<Duration>) -> Result<Response<PingResponse>, Status> {
        Err(Status::unimplemented("unused test RPC"))
    }
}

#[tokio::test]
async fn proxy_preserves_stream_messages_metadata_and_errors() {
    let backend_listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let backend_address = backend_listener.local_addr().unwrap();
    let backend_task = tokio::spawn(
        Server::builder()
            .add_service(CompactTxStreamerServer::new(TestBackend))
            .serve_with_incoming(TcpListenerStream::new(backend_listener)),
    );
    let upstream = CompactTxStreamerClient::connect(format!("http://{backend_address}"))
        .await
        .unwrap();
    let path =
        std::env::temp_dir().join(format!("darkroom-stream-test-{}.jsonl", std::process::id()));
    let _ = tokio::fs::remove_file(&path).await;
    let proxy = Proxy::new(upstream, &path, "stream-test".into())
        .await
        .unwrap();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let proxy_task = tokio::spawn(
        Server::builder()
            .add_service(CompactTxStreamerServer::new(proxy))
            .serve_with_incoming(TcpListenerStream::new(listener)),
    );
    let channel = Channel::from_shared(format!("http://{address}"))
        .unwrap()
        .timeout(std::time::Duration::from_secs(10))
        .connect()
        .await
        .unwrap();
    let mut client = CompactTxStreamerClient::new(channel);
    for height in [1, 2] {
        let mut request = Request::new(BlockRange {
            start: Some(BlockId {
                height,
                hash: vec![],
            }),
            ..Default::default()
        });
        request
            .metadata_mut()
            .insert("x-roundtrip", "present".parse().unwrap());
        let response = client.get_block_range(request).await.unwrap();
        assert_eq!(response.metadata().get("x-initial").unwrap(), "kept");
        let mut stream = response.into_inner();
        assert_eq!(stream.message().await.unwrap().unwrap().height, 17);
        if height == 1 {
            assert!(stream.message().await.unwrap().is_none());
            assert_eq!(
                stream
                    .trailers()
                    .await
                    .unwrap()
                    .unwrap()
                    .get("x-trailer")
                    .unwrap(),
                "kept"
            );
        } else {
            let status = stream.message().await.unwrap_err();
            assert_eq!(status.code(), tonic::Code::Unavailable);
            assert_eq!(status.message(), "backend ended");
            assert_eq!(status.metadata().get("x-trailer").unwrap(), "kept");
        }
    }
    let log = tokio::fs::read_to_string(&path).await.unwrap();
    assert_eq!(log.lines().count(), 2);
    for line in log.lines() {
        let event: serde_json::Value = serde_json::from_str(line).unwrap();
        assert_eq!(event["method"], "GetBlockRange");
        assert_eq!(event["session"], "stream-test");
    }
    proxy_task.abort();
    backend_task.abort();
    let _ = proxy_task.await;
    let _ = backend_task.await;
    tokio::fs::remove_file(path).await.unwrap();
}
