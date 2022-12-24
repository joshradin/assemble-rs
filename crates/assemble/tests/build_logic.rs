use assemble::builders::js::JavascriptBuilder;
use assemble::dev::FreightRunnerBuilder;

#[test]
fn build_logic() {
    let freight = FreightRunnerBuilder::<JavascriptBuilder>::new().build();
    println!("assemble home = {:?}", freight.assemble_home());
}
