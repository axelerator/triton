use wasm_bindgen::prelude::*;
use web_sys::{Element, HtmlElement};
use wasm_bindgen::JsCast;

// Called when the wasm module is instantiated
#[wasm_bindgen(start)]
pub fn main() -> Result<(), JsValue> {
    // Use `web_sys`'s global `window` function to get a handle on the global
    // window object.
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");


    Ok(())
}

#[wasm_bindgen]
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[wasm_bindgen]
pub fn render_all(selector: &str) -> u32 {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let elements = document.query_selector_all(selector);

    match elements {
        Ok(node_list) => {
            let mut num = 0;
            for i in 0..node_list.length() {
                let node = node_list.get(i).unwrap();
                let e = node.dyn_into::<web_sys::HtmlElement>().expect("query_selector_all only returns elements");

                let doc = core::sequence_diagram::render(&e.text_content().unwrap()).unwrap();

                e.set_inner_html(&doc.to_string().as_str());
                num += 1;
            }
            num
        },
        Err(_) => 0
    }
}

