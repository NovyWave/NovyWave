use moon::*;

async fn frontend() -> Frontend {
    Frontend::new()
        .title("NovyWave - Fast2D Demo")
        .index_by_robots(false)
}

async fn up_msg_handler(_: UpMsgRequest<()>) {}

#[moon::main]
async fn main() -> std::io::Result<()> {
    start(frontend, up_msg_handler, |_| {}).await
}
