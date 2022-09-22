use assemble::dev::FreightRunnerBuilder;

#[test]
fn build_logic() {
    let freight = FreightRunnerBuilder::new().build();
    println!("assemble home = {:?}", freight.assemble_home());
}
