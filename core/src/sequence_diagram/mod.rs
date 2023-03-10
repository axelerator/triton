pub mod parser;
pub mod render;

pub use parser::*;
pub use render::*;
use svg::node::element::SVG;

pub fn render(src: &str) -> Result<SVG, String>{
    let svg_config =  crate::sequence_diagram::SvgConfig {
        max_participant_head_length: 5,
        max_msg_label_length: 60,
        font_scale_factor: 1.2,
        msg_gutter: 20.0,
        font_size: 10.0,
        padding: 5.0,
        corner_radius: 2.0,
    };

    match crate::sequence_diagram::parser::parse(src.to_string()) {
        Ok(diagram) => {
            Ok(crate::sequence_diagram::render::to_svg(&diagram, &svg_config))
        }
        Err(e) => {
            Err(format!("Error: {e:?}").to_owned())
        }
    }
}
