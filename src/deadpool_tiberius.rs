use async_std::net::TcpStream;
use async_trait::async_trait;

type RecycleResult = deadpool::managed::RecycleResult<tiberius::error::Error>;
pub type Pool = deadpool::managed::Pool<tiberius::Client<TcpStream>, tiberius::error::Error>;

pub struct Manager {
  pub config : tiberius::Config,
}

impl Manager {
    pub fn new(config: tiberius::Config) -> Self {
        Self {config}
    }
}

#[async_trait]
impl deadpool::managed::Manager<tiberius::Client<TcpStream>, tiberius::error::Error>  for Manager
{
    async fn create(&self) -> Result<tiberius::Client<TcpStream>, tiberius::error::Error> {
        let tcp = TcpStream::connect(self.config.get_addr()).await?;
        tcp.set_nodelay(true)?;
        let conn = tiberius::Client::connect(self.config.clone(), tcp).await?;
        Ok(conn)
    }
    async fn recycle(&self, conn: &mut tiberius::Client<TcpStream>) -> RecycleResult {
        conn.simple_query("SELECT 1").await?;
        Ok(())
    }
}
