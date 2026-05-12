fn main() {
    embed_resource::compile("resources/app.rc", embed_resource::NONE)
        .manifest_required()
        .expect("failed to compile Windows resources");
}
