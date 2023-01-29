
fn main() {

    // "Alice->Bob long name:Solving the system each time make for faster updates and allow to keep the solver in a consinstent state. However, the variable values are not updated automatically and you need to ask the solver to perform this operation before reading the values as illustrated below\nJohn->Bob long name:iiiiiiiiiiiiiiiiiiiii\nBob long name->John:It's Alice\nBob long name->Alice:I'm fine\n".to_string(),

    let _src = r#"
        participant John
        Alice->+John: Hello John, how are you?
        Alice->+John: John, can you hear me?
        John->-Alice: Hi Alice, I can hear you!
        John->-Alice: I feel great!
    "#;
    let src = r#"
        Alice->+John: Hello John, how are you?
        Note left of John: yeah
    "#;


    core::sequence_diagram::render(src)
}
