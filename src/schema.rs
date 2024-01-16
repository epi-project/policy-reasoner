// @generated automatically by Diesel CLI.

diesel::table! {
    active_version (version, activated_on) {
        version -> BigInt,
        activated_on -> Timestamp,
        activated_by -> Text,
        deactivated_on -> Nullable<Timestamp>,
        deactivated_by -> Nullable<Text>,
    }
}

diesel::table! {
    policies (version) {
        version -> BigInt,
        description -> Text,
        version_description -> Text,
        creator -> Text,
        created_at -> BigInt,
        content -> Text,
        reasoner_connector_context -> Text,
    }
}

diesel::joinable!(active_version -> policies (version));

diesel::allow_tables_to_appear_in_same_query!(
    active_version,
    policies,
);
