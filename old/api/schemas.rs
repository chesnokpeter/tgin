use serde::Deserialize;

fn default_sublevel() -> i8 {
    0
}

#[derive(Deserialize, Debug)]
pub struct AddWebhookRouteSch {
    #[serde(default)]
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct AddLongpullRouteSch {
    #[serde(default)]
    pub path: String,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum AddRouteTypeSch {
    Webhook(AddWebhookRouteSch),
    Longpull(AddLongpullRouteSch),
}

#[derive(Deserialize, Debug)]
pub struct AddRouteSch {
    #[serde(flatten)]
    pub typee: AddRouteTypeSch,
    #[serde(default = "default_sublevel")]
    pub sublevel: i8,
}

#[derive(Deserialize, Debug)]
pub struct RmWebhookRouteSch {
    #[serde(default)]
    pub url: String,
}

#[derive(Deserialize, Debug)]
pub struct RmLongpullRouteSch {
    #[serde(default)]
    pub path: String,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type")]
pub enum RmRouteTypeSch {
    Webhook(RmWebhookRouteSch),
    Longpull(RmLongpullRouteSch),
}

#[derive(Deserialize, Debug)]
pub struct RmRouteSch {
    #[serde(flatten)]
    pub typee: RmRouteTypeSch,
}
