//! `arks serve` forge rulesets on push: adapts the forge's ruleset evaluation
//! ([`arks_hub::push_verdicts`]) to the Git server's push policy seam
//! ([`arks_server::PushPolicy`]). Contract: [[sot:root/arks-hub-product-intent]] §17–22.

use arks_hub::{ForgeServeOptions, PushRefUpdate, push_verdicts};
use arks_repo::Repository;
use arks_server::{PushPolicy, Pusher, RefUpdate};
use std::path::PathBuf;
use std::sync::Arc;

/// Push policy reading the forge store of one `arks serve` forge module.
pub(crate) struct ForgeRulesetPolicy {
    data_dir: PathBuf,
    git_root: Option<PathBuf>,
}

impl PushPolicy for ForgeRulesetPolicy {
    fn check(
        &self,
        repo: &Repository,
        pusher: &Pusher,
        updates: &[RefUpdate],
    ) -> Vec<Option<String>> {
        let ups: Vec<PushRefUpdate> = updates
            .iter()
            .map(|u| PushRefUpdate {
                name: u.name.clone(),
                old: (!u.is_create()).then_some(u.old),
                new: (!u.is_delete()).then_some(u.new),
            })
            .collect();
        push_verdicts(
            &self.data_dir,
            self.git_root.as_deref(),
            repo.git_dir(),
            pusher.verified_user.as_deref(),
            pusher.authorization.as_deref(),
            &ups,
        )
    }
}

/// Install the forge ruleset check on this process's receive-pack paths.
pub(crate) fn install(opts: &ForgeServeOptions) {
    arks_server::install_push_policy(Some(Arc::new(ForgeRulesetPolicy {
        data_dir: opts.data_dir.clone(),
        git_root: opts.git_root.clone(),
    })));
}
