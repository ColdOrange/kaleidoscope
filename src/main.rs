extern crate kaleidoscope;

use kaleidoscope::jit::JIT;

fn main() {
    let mut jit = JIT::new(r"
        def test(x) (1+2+x)*(x+(1+2));
        test(3);
    ");

    jit.run();
}
