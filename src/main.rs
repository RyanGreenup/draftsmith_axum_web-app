use clap::Parser;
pub mod flash;
pub mod html_builder;
pub mod server;
// TODO this should be a module of server
mod static_files;
mod template_context;
mod templates;

pub const MAX_ITEMS_PER_PAGE: usize = 50;

/// Simple program to greet a person or serve an API
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
enum Command {
    /// Serve the Web Application
    Serve {
        /// Host for the API server (default: "localhost")
        #[arg(short = 'S', long, default_value_t = String::from("localhost"), required = false)]
        api_host: String,

        /// Port for the API server (default: 8080)
        #[arg(short = 'P', long, default_value_t = 37240, required = false)]
        api_port: u16,

        /// Scheme for the API (http or https) (default: "http")
        #[arg(short = 't', long, default_value_t = String::from("http"), required = false)]
        api_scheme: String,

        /// Port for the Web App
        #[arg(short = 'p', long, default_value_t = String::from("8080"), required = false)]
        port: String,

        /// Host for the Web App
        #[arg(short = 's', long, default_value_t = String::from("0.0.0.0"), required = false)]
        host: String,
    },
}

fn main() {
    let command = Command::parse();

    match &command {
        Command::Serve {
            api_host,
            api_port,
            api_scheme,
            host,
            port,
        } => {
            // Placeholder for serving logic
            println!(
                "Serving Web App at {host}:{port} using API {api_scheme}://{api_host}:{api_port}"
            );
            server::serve(api_scheme, api_host, api_port, host, port);
        }
    }
}
