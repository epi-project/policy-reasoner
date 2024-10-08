use std::net::SocketAddr;

use clap::Parser;

/***** ARGUMENTS *****/
/// Defines the arguments for the `policy-reasoner` server.
#[derive(Debug, Parser)]
pub struct Arguments {
    /// Whether to enable full debugging
    #[clap(long, global = true, help = "If given, enables more verbose debugging.")]
    pub trace: bool,

    /// The address on which to bind ourselves.
    #[clap(short, long, env, default_value = "127.0.0.1:3030", help = "The address on which to bind the server.")]
    pub address: SocketAddr,

    /// Shows the help menu for the state resolver.
    #[clap(long, help = "If given, shows the possible arguments to pass to the state resolver plugin in '--state-resolver'.")]
    pub help_state_resolver: bool,

    /// Arguments specific to the state resolver.
    #[clap(
        short,
        long,
        env,
        help = "Arguments to pass to the current state resolver plugin. To find which are possible, see '--help-state-resolver'."
    )]
    pub state_resolver: Option<String>,

    /// Shows the help menu for the reasoner connector.
    #[clap(long, help = "If given, shows the possible arguments to pass to the reasoner connector plugin in '--reasoner-connector'.")]
    pub help_reasoner_connector: bool,

    /// Arguments specific to the state resolver.
    #[clap(
        short,
        long,
        env,
        help = "Arguments to pass to the current reasoner connector plugin. To find which are possible, see '--help-reasoner-connector'."
    )]
    pub reasoner_connector: Option<String>,
}
