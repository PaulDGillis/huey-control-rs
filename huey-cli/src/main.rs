use huey_core::{ light::{ Color, Light, ColorXY }, HueBridge };
use clap::{ Parser, Subcommand, Args };

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(name = "light-rs")]
#[command(about = "A tool for controlling hue lights.", long_about = None)]
struct LightCli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Discover bridge ip", arg_required_else_help = false)]
    Discover,
    #[command(about = "Request api key from bridge", arg_required_else_help = false)]
    Pair,
    Light(LightArgs)
}

#[derive(Debug, Args)]
#[command(about = "Control lights on bridge", arg_required_else_help = true)]
struct LightArgs {
    #[arg(short = 'b', long = "bridge")]
    bridge: String,

    #[arg(short = 'k', long = "key")]
    key: String,

    #[command(subcommand)]
    command: Option<LightCommands>
}

#[derive(Debug, Subcommand)]
enum LightCommands {
    List,
    Power {
        light_id: String,

        #[arg(long)]
        on: bool
    },
    Color { 
        light_id: String,
        #[arg(short)]
        x: Option<f64>,
        #[arg(short)]
        y: Option<f64>,
        #[arg(short, long)]
        brightness: Option<f64>
    },
}

#[tokio::main]
async fn main() {
    let args = LightCli::parse();
    match args.command {
        Commands::Discover => {
            let result = HueBridge::discover().await;
            println!("{:?}", result);
        },
        Commands::Pair => {
            let result = HueBridge::pair("10.0.99.56".into()).await;
            println!("{:?}", result);
        },
        Commands::Light(light_args) => {
            let bridge = HueBridge::new(light_args.key, light_args.bridge);
            let light_command = light_args.command.unwrap_or(LightCommands::List);
            match light_command {
                LightCommands::List => {
                    let result = Light::list_lights(&bridge).await;
                    println!("{:?}", result);
                },
                LightCommands::Power { light_id, on } => {
                    let result = Light::toggle_power_id(light_id, on)
                        .on(&bridge)
                        .await;
                    println!("{:?}", result);
                },
                LightCommands::Color { light_id, x, y, brightness } => {
                    let mut color_opt = None;
                    if let (Some(x), Some(y), Some(brightness)) = (x, y, brightness) {
                        color_opt = Some(Color::XY(ColorXY { x, y, bri: brightness }));
                    }

                    let result = Light::change_color_id(light_id, color_opt, brightness, None)
                        .on(&bridge)
                        .await;
                    println!("{:?}", result);
                },
            }
        }
    }
}
