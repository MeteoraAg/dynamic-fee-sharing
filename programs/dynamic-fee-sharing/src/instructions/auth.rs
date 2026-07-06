use anchor_lang::prelude::*;

pub mod admin {
    use super::*;

    #[cfg(feature = "local")]
    pub const ADMINS: [Pubkey; 1] = [pubkey!("bossj3JvwiNK7pvjr149DqdtJxf2gdygbcmEPTkb2F1")];

    #[cfg(not(feature = "local"))]
    pub const ADMINS: [Pubkey; 2] = [
        pubkey!("5unTfT2kssBuNvHPY6LbJfJpLqEcdMxGYLWHwShaeTLi"),
        pubkey!("DHLXnJdACTY83yKwnUkeoDjqi4QBbsYGa1v8tJL76ViX"),
    ];
}

pub fn assert_eq_admin(admin: Pubkey) -> bool {
    crate::instructions::auth::admin::ADMINS
        .iter()
        .any(|predefined_admin| predefined_admin.eq(&admin))
}
