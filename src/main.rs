extern crate actix;
extern crate actix_web;
extern crate log;
extern crate pretty_env_logger;

use actix::prelude::*;
use actix_web::{fs, http, middleware, server, ws, App, Error, HttpRequest, HttpResponse};

use std::time::{Duration, Instant};

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// do WS handshake and start actor
fn ws_index(r: &HttpRequest) -> Result<HttpResponse, Error> {
    ws::start(r, Ws::new())
}

// this is a long-running connection
struct Ws {
    // Client must ping once per CLIENT_TIMEOUT
    // otherwise we drop
    hb: Instant,
}

impl Actor for Ws {
    type Context = ws::WebsocketContext<Self>;

    /// called on actor start - starts the heartbeat process
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);
    }
}

impl StreamHandler<ws::Message, ws::ProtocolError> for Ws {
    fn handle(&mut self, msg: ws::Message, ctx: &mut Self::Context) {
        println!("WS: {:?}", msg);
        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(text) => ctx.text(text),
            ws::Message::Binary(bin) => ctx.binary(bin),
            ws::Message::Close(_) => {
                // we're ignoreing the optional reason
                ctx.stop();
            }
        }
    }
}

impl Ws {
    fn new() -> Self {
        Self { hb: Instant::now() }
    }

    /// send ping to client every second, checks heartbeats from client
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |actor, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(actor.hb) > CLIENT_TIMEOUT {
                // handle a timeout
                println!("Websocket Client heartbeat failed, disconnecting!");

                // stop actor
                ctx.stop();

                // don't ping
                return;
            }

            ctx.ping("");
        });
    }
}

fn main() {
    ::std::env::set_var("RUST_LOG", "actix_web=info");
    pretty_env_logger::init();
    let sys = actix::System::new("protoqueue-rs");
    server::new(|| {
        App::new()
            .middleware(middleware::Logger::default())
            .resource("/ws/", |r| r.method(http::Method::GET).f(ws_index))
            .handler(
                "/",
                fs::StaticFiles::new("static/")
                    .unwrap()
                    .index_file("index.html"),
            )
    }).bind("127.0.0.1:8080")
    .unwrap()
    .start();

    println!("Started http server: 127.0.0.1:8080");
    let _ = sys.run();
}
