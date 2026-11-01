//! adminsvr roles (ADMINSVR.md decision 11 and section 2). Enforced here; the app only mirrors.
use anyhow::{bail, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    Sre,
    Admin,
    Owner,
}

impl Role {
    pub fn parse(s: &str) -> Option<Role> {
        match s {
            "sre" => Some(Role::Sre),
            "admin" => Some(Role::Admin),
            "owner" => Some(Role::Owner),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Sre => "sre",
            Role::Admin => "admin",
            Role::Owner => "owner",
        }
    }
}

/// Default role for new users.
pub const DEFAULT_ROLE: Role = Role::Sre;

/// Create a user with `role`.
pub fn can_create(actor: Role, role: Role) -> Result<()> {
    if actor < Role::Admin {
        bail!("only owners and admins create users");
    }
    if role == Role::Owner && actor != Role::Owner {
        bail!("only an owner can create an owner");
    }
    Ok(())
}

/// Change `target`'s role from `from` to `to`. `active_owners` counts active owners now.
pub fn can_set_role(
    actor: Role,
    is_self: bool,
    from: Role,
    to: Role,
    active_owners: i64,
) -> Result<()> {
    if from == Role::Owner && to != Role::Owner && active_owners <= 1 {
        bail!("the last owner can't be demoted");
    }
    if is_self {
        if to > from {
            bail!("you can't promote yourself");
        }
        return Ok(()); // self-demotion
    }
    if actor < Role::Admin {
        bail!("only owners and admins change roles");
    }
    if to == Role::Owner && actor != Role::Owner {
        bail!("admins can't grant owner");
    }
    if from == Role::Owner && actor != Role::Owner {
        bail!("admins can't change an owner");
    }
    Ok(())
}

/// Disable/enable or otherwise edit `target` (role `target_role`).
pub fn can_edit(actor: Role, is_self: bool, target_role: Role) -> Result<()> {
    if is_self {
        return Ok(());
    }
    if actor < Role::Admin {
        bail!("only owners and admins edit users");
    }
    if target_role == Role::Owner && actor != Role::Owner {
        bail!("admins can't edit an owner");
    }
    Ok(())
}

pub fn can_disable(
    actor: Role,
    is_self: bool,
    target_role: Role,
    active_owners: i64,
) -> Result<()> {
    if target_role == Role::Owner && active_owners <= 1 {
        bail!("the last owner can't be disabled");
    }
    can_edit(actor, is_self, target_role)
}

pub fn require(actor: Role, min: Role) -> Result<()> {
    if actor < min {
        bail!("needs role {} (you have {})", min.as_str(), actor.as_str());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Role::*;
    use super::*;
    #[test]
    fn owner_rules() {
        assert!(can_create(Admin, Sre).is_ok());
        assert!(can_create(Admin, Admin).is_ok());
        assert!(can_create(Admin, Owner).is_err());
        assert!(can_create(Sre, Sre).is_err());
        assert!(can_create(Owner, Owner).is_ok());
        // admins can't promote to owner or touch owners
        assert!(can_set_role(Admin, false, Sre, Owner, 2).is_err());
        assert!(can_set_role(Admin, false, Owner, Admin, 2).is_err());
        assert!(can_set_role(Admin, false, Sre, Admin, 1).is_ok());
        assert!(can_set_role(Admin, false, Admin, Sre, 1).is_ok());
        // self: demote ok, promote never
        assert!(can_set_role(Admin, true, Admin, Owner, 1).is_err());
        assert!(can_set_role(Sre, true, Sre, Admin, 1).is_err());
        assert!(can_set_role(Admin, true, Admin, Sre, 1).is_ok());
        // last owner
        assert!(can_set_role(Owner, true, Owner, Admin, 1).is_err());
        assert!(can_set_role(Owner, true, Owner, Admin, 2).is_ok());
        assert!(can_set_role(Owner, false, Owner, Sre, 1).is_err());
        assert!(can_disable(Owner, false, Owner, 1).is_err());
        assert!(can_disable(Admin, false, Owner, 3).is_err());
        assert!(can_disable(Owner, false, Owner, 2).is_ok());
        assert!(can_edit(Sre, false, Sre).is_err());
    }
}
