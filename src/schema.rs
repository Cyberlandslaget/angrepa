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
