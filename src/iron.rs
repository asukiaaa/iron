//! Exposes the `Iron` type, the main entrance point of the
//! `Iron` library.

use std::io::net::ip::{SocketAddr, IpAddr};
use std::sync::Arc;

use http::server as http;
use super::{Request, Handler};

use super::response::HttpResponse;
use super::request::HttpRequest;

/// The primary entrance point to `Iron`, a `struct` to instantiate a new server.
///
/// The server can be made with a specific `Chain` (using `from_chain`)
/// or with a new `Chain` (using `new`). `Iron` is used to manage the server
/// processes:
/// `Iron.chain.link` is used to add new `Middleware`, and
/// `Iron.listen` is used to kick off a server process.
///
/// `Iron` contains the `Chain` which holds the `Middleware` necessary to run a server.
/// `Iron` is the main interface to adding `Middleware`, and has `Chain` as a
/// public field (for the sake of extensibility).
pub struct Iron<H> {
    /// Add `Middleware` to the `Iron's` `chain` so that requests
    /// are passed through those `Middleware`.
    /// `Middleware` is added to the chain with with `chain.link`.
    pub handler: H,
}

// The struct which actually listens and serves requests.
struct IronListener<H> {
    handler: Arc<H>,
    ip: IpAddr,
    port: u16
}

impl<H: Send + Sync> Clone for IronListener<H> {
    fn clone(&self) -> IronListener<H> {
        IronListener {
            handler: self.handler.clone(),
            ip: self.ip.clone(),
            port: self.port.clone()
        }
    }
}

impl<H: Handler> Iron<H> {
    /// Kick off the server process.
    ///
    /// Call this once to begin listening for requests on the server.
    /// This is a blocking operation, and is the final op that should be called
    /// on the `Iron` instance. Once `listen` is called, requests will be
    /// handled as defined through the `Iron's` `chain's` `Middleware`.
    pub fn listen(self, ip: IpAddr, port: u16) {
        use http::server::Server;

        IronListener {
            handler: Arc::new(self.handler),
            ip: ip,
            port: port
        }.serve_forever();
    }

    /// Instantiate a new instance of `Iron`.
    ///
    /// This will create a new `Iron`, the base unit of the server.
    #[inline]
    pub fn around(handler: H) -> Iron<H> {
        Iron { handler: handler }
    }
}

impl<H: Handler> http::Server for IronListener<H> {
    fn get_config(&self) -> http::Config {
        http::Config {
            bind_address: SocketAddr {
                ip: self.ip,
                port: self.port
            }
        }
    }

    fn handle_request(&self, http_req: HttpRequest, http_res: &mut HttpResponse) {
        // Create wrapper Request and Response
        let mut req = match Request::from_http(http_req) {
            Ok(req) => req,
            Err(e) => {
                error!("Error getting request: {}", e);
                http_res.status = ::http::status::InternalServerError;
                let _ = http_res.write(b"Internal Server Error");
                return;
            }
        };

        // Dispatch the request
        let res = self.handler.call(&mut req);

        match res {
                    // Write the response back to http_res
            Ok(res) => res.write_back(http_res),
            Err(e) => {
                error!("Error handling {}: {}", req, e);
                http_res.status = ::http::status::InternalServerError;
                let _ = http_res.write(b"Internal Server Error");
            }
        }
    }
}
