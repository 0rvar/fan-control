fn main() {
    dotenv_build::output(dotenv_build::Config::default()).unwrap();
    embuild::espidf::sysenv::output();
}
