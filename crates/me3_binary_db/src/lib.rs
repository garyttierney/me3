use me3_binary::Program;

#[salsa::jar(db = Db)]
pub struct Jar();

pub trait Db: salsa::DbWithJar<Jar> {
    fn program(&self) -> Program<'_>;
}

#[derive(Default)]
#[salsa::db(Jar)]
struct OnlineProgramDatabase {
    storage: salsa::Storage<Self>,
}

impl salsa::Database for OnlineProgramDatabase {}
impl Db for OnlineProgramDatabase {
    fn program(&self) -> Program<'_> {
        unsafe { Program::current() }
    }
}
