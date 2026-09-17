use presence_organ_sdk::{organ_ok, OrganArgs};

fn main() {
    let args = OrganArgs::from_env();
    let op = args.op();

    let resp = organ_ok!(
        "operation" => op,
        "organ" => "template"
    );
    resp.print_and_exit();
}