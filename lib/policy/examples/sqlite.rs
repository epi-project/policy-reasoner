use policy::{sqlite::SqlitePolicyDataStore, policy::PolicyDataAccess};


fn main() {
    // /// DATA STORE TEST

    let policy_data_store = &mut SqlitePolicyDataStore::new("./data/policies.db");

    // let content = serde_json::value::RawValue::from_string("\"hallo\"".into()).unwrap();

    // let version = Policy {
    //     description: String::from("Dit is een omschrijving"),
    //     version: PolicyVersion {
    //         creator: String::from("Bas Kloosterman"),
    //         created_at: Local::now(),
    //         version: None,
    //         version_description: String::from("Dit is een versie omschrijving"),
    //     },
    //     content: vec![PolicyContent {
    //         reasoner: String::from("eflint"),
    //         reasoner_version: String::from("v1.0.0"),
    //         content: content,
    //     }],
    // };

    // let _p = policy_data_store.add_version(version).unwrap();

    // let latest = policy_data_store.get_most_recent().unwrap();

    // let latest_json = serde_json::to_string_pretty(&latest).unwrap();

    // println!("new policy: {latest_json}");

    // let four = policy_data_store.get_version("4").unwrap();

    // let four_json = serde_json::to_string_pretty(&four).unwrap();

    // println!("4 policy: {four_json}");

    // let versions = policy_data_store.get_versions().unwrap();
    // let versions_json = serde_json::to_string_pretty(&versions).unwrap();

    // println!("versions: {versions_json}");

    // policy_data_store.set_active(String::from("4"), String::from("Bas Kloosterman"));

    match policy_data_store.get_active() {
        Ok(policy) => {
            println!("{}", serde_json::to_string_pretty(&policy).unwrap());
        },
        Err(err) => println!("Err: {err}")
    }
}