use k8s_openapi::api::apps::v1::Deployment;
use k8s_openapi::api::core::v1::ConfigMap;
use k8s_openapi::api::networking::v1::Ingress;
use kube::api::ListParams;
use kube::{
    api::{Api, ResourceExt},
    Client,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let kube_client = Client::try_default().await?;
    let config_maps: Api<ConfigMap> = Api::namespaced(kube_client.clone(), "viceapps");
    let ingresses: Api<Ingress> = Api::namespaced(kube_client.clone(), "viceapps");
    let deployments: Api<Deployment> = Api::namespaced(kube_client.clone(), "viceapps");

    for cm in config_maps.list(&ListParams::default()).await? {
        println!("configmap: {}", cm.name_any());
    }

    for i in ingresses.list(&ListParams::default()).await? {
        println!("ingress: {}", i.name_any());
    }

    for d in deployments.list(&ListParams::default()).await? {
        println!("deployment: {}", d.name_any());
    }

    Ok(())
}
