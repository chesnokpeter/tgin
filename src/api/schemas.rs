use serde::Deserialize;

fn default_sublevel() -> i8 {
    0
}


#[derive(Deserialize, Debug)]
pub struct AddWebhookRoute {
    #[serde(default)]
    pub url: String,
    #[serde(default = "default_sublevel")]
    pub sublevel: i8
}


#[derive(Deserialize, Debug)]
pub struct AddLongpullRoute {
    #[serde(default)]
    path: String,
    #[serde(default = "default_sublevel")]
    sublevel: i8
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AddRoute {
    Webhook(AddWebhookRoute),
    Longpull(AddLongpullRoute),
}