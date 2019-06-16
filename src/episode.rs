#[derive(Default, Clone, Debug)]
pub struct Episode {
    pub castid: i32,
    pub episodeid: i32,
    pub title: String,
    pub epurl: String,
    pub enctype: String,
    pub status: String,
    pub eplength: i32,
    pub epfirstattempt: Option<i32>,
    pub eplastattempt: Option<i32>,
    pub epfailedattempts: i32,
    pub epguid: Option<String>,
}
