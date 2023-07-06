use olive_proto::{
    process_instance_service_server::ProcessInstanceService,
    server::process_definitions_service_server::ProcessDefinitionsService,
    CancelProcessInstanceRequest, CancelProcessInstanceResponse, DeleteDefinitionsRequest,
    DeleteDefinitionsResponse, DeployDefinitionsRequest, DeployDefinitionsResponse,
    ExecuteDefinitionsRequest, ExecuteDefinitionsResponse, GetDefinitionsRequest,
    GetDefinitionsResponse, GetProcessInstanceRequest, GetProcessInstanceResponse,
    ListDefinitionsRequest, ListDefinitionsResponse, ListProcessInstanceRequest,
    ListProcessInstanceResponse, ProcessInstance, ProcessDefinitions,
};
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct OliveServer {}

#[tonic::async_trait]
impl ProcessDefinitionsService for OliveServer {
    async fn deploy_definitions(
        &self,
        request: Request<DeployDefinitionsRequest>,
    ) -> Result<Response<DeployDefinitionsResponse>, Status> {
        println!("Got a request from {:?}", request.remote_addr());
        let req = request.into_inner();
        let _ = req.name;

        let rsp = DeployDefinitionsResponse {
            definitions: Some(ProcessDefinitions::default()),
        };
        Ok(Response::new(rsp))
    }

    async fn list_definitions(
        &self,
        request: Request<ListDefinitionsRequest>,
    ) -> Result<Response<ListDefinitionsResponse>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let rsp = ListDefinitionsResponse {
            definitions: vec![],
        };
        Ok(Response::new(rsp))
    }

    async fn get_definitions(
        &self,
        request: Request<GetDefinitionsRequest>,
    ) -> Result<Response<GetDefinitionsResponse>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let rsp = GetDefinitionsResponse {
            definitions: Some(ProcessDefinitions::default()),
        };
        Ok(Response::new(rsp))
    }

    async fn delete_definitions(
        &self,
        request: Request<DeleteDefinitionsRequest>,
    ) -> Result<Response<DeleteDefinitionsResponse>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let rsp = DeleteDefinitionsResponse {};
        Ok(Response::new(rsp))
    }

    async fn execute_definitions(
        &self,
        request: Request<ExecuteDefinitionsRequest>,
    ) -> Result<Response<ExecuteDefinitionsResponse>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let rsp = ExecuteDefinitionsResponse {
            instance: Some(ProcessInstance::default()),
        };
        Ok(Response::new(rsp))
    }
}

#[tonic::async_trait]
impl ProcessInstanceService for OliveServer {
    async fn list_process_instance(
        &self,
        request: Request<ListProcessInstanceRequest>,
    ) -> Result<Response<ListProcessInstanceResponse>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let rsp = ListProcessInstanceResponse { instances: vec![] };
        Ok(Response::new(rsp))
    }

    async fn get_process_instance(
        &self,
        request: Request<GetProcessInstanceRequest>,
    ) -> Result<Response<GetProcessInstanceResponse>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let rsp = GetProcessInstanceResponse {
            instance: Some(ProcessInstance::default()),
        };
        Ok(Response::new(rsp))
    }

    async fn cancel_process_instance(
        &self,
        request: Request<CancelProcessInstanceRequest>,
    ) -> Result<Response<CancelProcessInstanceResponse>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let rsp = CancelProcessInstanceResponse {
            instance: Some(ProcessInstance::default()),
        };
        Ok(Response::new(rsp))
    }
}
