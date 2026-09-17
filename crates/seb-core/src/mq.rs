use crate::config::AppConfig;
use amqprs::callbacks::{DefaultChannelCallback, DefaultConnectionCallback};
use amqprs::channel::{BasicPublishArguments, Channel};
use amqprs::connection::{Connection, OpenConnectionArguments};
use amqprs::BasicProperties;

#[derive(Debug, thiserror::Error)]
pub enum MqError {
    #[error("连接 RabbitMQ 失败: {0}")]
    Connect(String),
    #[error("发送失败: {0}")]
    Publish(String),
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PublishRecord {
    pub exchange: String,
    pub routing_key: String,
    pub body: String,
    pub reconnected: bool,
}

pub struct Publisher {
    cfg: AppConfig,
    conn: Option<Connection>,
    chan: Option<Channel>,
}

impl Publisher {
    pub fn new(cfg: AppConfig) -> Self {
        Self {
            cfg,
            conn: None,
            chan: None,
        }
    }

    pub fn config(&self) -> &AppConfig {
        &self.cfg
    }

    pub async fn set_config(&mut self, cfg: AppConfig) {
        self.cfg = cfg;
    }

    pub fn is_connected(&self) -> bool {
        matches!((&self.conn, &self.chan), (Some(c), Some(ch)) if c.is_open() && ch.is_open())
    }

    pub async fn disconnect(&mut self) {
        if let Some(ch) = self.chan.take() {
            let _ = ch.close().await;
        }
        if let Some(conn) = self.conn.take() {
            let _ = conn.close().await;
        }
    }

    async fn channel(&mut self) -> Result<&Channel, MqError> {
        if self.is_connected() {
            return Ok(self.chan.as_ref().expect("已校验存在"));
        }

        self.disconnect().await;

        let args = OpenConnectionArguments::new(
            crate::config::MQ_HOST,
            crate::config::MQ_PORT,
            crate::config::MQ_USERNAME,
            crate::config::MQ_PASSWORD,
        );
        let conn = Connection::open(&args)
            .await
            .map_err(|e| MqError::Connect(e.to_string()))?;
        conn.register_callback(DefaultConnectionCallback)
            .await
            .map_err(|e| MqError::Connect(e.to_string()))?;

        let chan = conn
            .open_channel(None)
            .await
            .map_err(|e| MqError::Connect(e.to_string()))?;
        chan.register_callback(DefaultChannelCallback)
            .await
            .map_err(|e| MqError::Connect(e.to_string()))?;

        self.conn = Some(conn);
        self.chan = Some(chan);
        Ok(self.chan.as_ref().expect("刚刚赋值"))
    }

    pub async fn publish(
        &mut self,
        exchange: &str,
        routing_key: &str,
        body: &str,
    ) -> Result<PublishRecord, MqError> {
        let mut last_err = None;

        for attempt in 0..2 {
            let args = BasicPublishArguments::new(exchange, routing_key);
            let result = match self.channel().await {
                Ok(ch) => ch
                    .basic_publish(BasicProperties::default(), body.as_bytes().to_vec(), args)
                    .await
                    .map_err(|e| MqError::Publish(e.to_string())),
                Err(e) => Err(e),
            };

            match result {
                Ok(()) => {
                    return Ok(PublishRecord {
                        exchange: exchange.to_string(),
                        routing_key: routing_key.to_string(),
                        body: body.to_string(),
                        reconnected: attempt > 0,
                    })
                }
                Err(e) => {
                    last_err = Some(e);
                    self.disconnect().await;
                }
            }
        }

        Err(last_err.unwrap_or_else(|| MqError::Publish("未知错误".into())))
    }
}
