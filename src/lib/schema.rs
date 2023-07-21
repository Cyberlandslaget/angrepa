// @generated automatically by Diesel CLI.

diesel::table! {
    exploits (id) {
        id -> Text,
        running -> Bool,
        attack_target -> Nullable<Text>,
        docker_image -> Text,
        exploit_kind -> Text,
    }
}

diesel::table! {
    flags (flag) {
        flag -> Text,
        tick -> Nullable<Int4>,
        stamp -> Nullable<Timestamp>,
        exploit_id -> Nullable<Text>,
        target_ip -> Nullable<Text>,
        flagstore -> Nullable<Text>,
        sent -> Bool,
        status -> Nullable<Text>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    exploits,
    flags,
);
