//! Locks: deciding whether a block set is locked, unlocking it through a
//! challenge, and refusing edits that would weaken a locked set.

use std::collections::HashSet;
use std::hash::Hash;

use argon2::{Algorithm, Argon2, Params, Version};
use serde::{Deserialize, Serialize};

use crate::geo::{LocationFix, Relation};
use crate::model::{BlockSet, Config, Lock, LockKind, SetId, UserScope};
use crate::schedule::{Now, Schedule};
use crate::text::{from_hex, to_hex};
use crate::CoreError;

/// Everything needed to decide whether a lock currently holds.
#[derive(Debug, Clone, Copy)]
pub struct LockContext<'a> {
    pub now: Now,
    /// Identifier of the current boot (`/proc/sys/kernel/random/boot_id`
    /// on Linux).
    pub boot_id: &'a str,
    pub location: LocationFix,
    /// Areas the platform knows the machine is inside by other means (Wi-Fi
    /// networks learned there), whatever `location` says.
    pub inside: &'a [crate::geo::GeoArea],
    /// Areas the platform knows the machine is away from (Wi-Fi works but
    /// none of the networks learned there is in range).
    pub outside: &'a [crate::geo::GeoArea],
}

pub fn is_locked(set: &BlockSet, ctx: &LockContext) -> bool {
    set.lock.as_ref().is_some_and(|lock| lock_holds(&lock.kind, ctx))
}

fn lock_holds(kind: &LockKind, ctx: &LockContext) -> bool {
    match kind {
        LockKind::Until { until } => ctx.now.unix < *until,
        LockKind::Schedule { schedule } => schedule.is_active(&ctx.now),
        LockKind::Password { .. } | LockKind::RandomText { .. } => true,
        LockKind::UntilRestart { boot_id } => boot_id == ctx.boot_id,
        LockKind::Location { area } => {
            if ctx.inside.contains(area) {
                true
            } else if ctx.outside.contains(area) {
                false
            } else {
                area.relation(ctx.location) != Relation::Outside
            }
        }
    }
}

/// Removes locks that can never hold again (deadline passed, machine
/// restarted) and applies due pending unlocks. A lock with an end time works
/// as a timer for its block, and an opened lock (password, text) frees it:
/// either way the block is turned off too. A restart lock only ends.
/// Returns true if anything changed. The daemon calls this periodically.
pub fn expire(set: &mut BlockSet, ctx: &LockContext) -> bool {
    if let Some(at) = set.pending_unlock_at
        && ctx.now.unix >= at
    {
        set.lock = None;
        set.pending_unlock_at = None;
        set.enabled = false;
        return true;
    }
    let (finished, turn_off) = match set.lock.as_ref().map(|l| &l.kind) {
        Some(LockKind::Until { until }) => (ctx.now.unix >= *until, true),
        Some(LockKind::UntilRestart { boot_id }) => (boot_id != ctx.boot_id, false),
        // A password no answer can match (Iqal 0.1.1-0.1.2 could store the
        // blank stand-in sent to non-administrators): the lock could never
        // be opened, so it is removed, as if opened.
        Some(LockKind::Password { hash }) => (!hash.is_usable(), true),
        _ => (false, false),
    };
    if finished {
        set.lock = None;
        if turn_off {
            set.enabled = false;
        }
    }
    apply_times(set, &ctx.now) || finished
}

/// A "during certain times" lock switches its block: on when its times
/// start, off when they end (in between, the block can be switched by
/// hand). Returns true if anything changed.
pub fn apply_times(set: &mut BlockSet, now: &Now) -> bool {
    match set.lock.as_ref().map(|l| &l.kind) {
        Some(LockKind::Schedule { schedule }) => {
            let inside = schedule.is_active(now);
            if set.times_on == Some(inside) {
                return false;
            }
            set.enabled = inside;
            set.times_on = Some(inside);
            true
        }
        _ => set.times_on.take().is_some(),
    }
}

/// What the user must do to open a lock early.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Challenge {
    /// Time, schedule, restart and location locks cannot be opened early.
    NotPossible,
    Password,
    /// Type exactly this text.
    RandomText(String),
}

pub fn challenge_for(lock: &Lock) -> Result<Challenge, CoreError> {
    Ok(match &lock.kind {
        LockKind::Password { .. } => Challenge::Password,
        LockKind::RandomText { length } => Challenge::RandomText(random_text(*length as usize)?),
        _ => Challenge::NotPossible,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnlockOutcome {
    Unlocked,
    /// The lock will be removed at this unix time.
    Pending { at: i64 },
}

/// Answers a challenge. `issued` is the random text that was shown to the
/// user for [`LockKind::RandomText`] locks.
pub fn try_unlock(
    set: &mut BlockSet,
    answer: &str,
    issued: Option<&str>,
    now: &Now,
) -> Result<UnlockOutcome, CoreError> {
    let Some(lock) = &set.lock else { return Ok(UnlockOutcome::Unlocked) };
    let correct = match &lock.kind {
        LockKind::Password { hash } => hash.verify(answer),
        LockKind::RandomText { length } => {
            issued.is_some_and(|t| t.chars().count() == *length as usize && constant_time_eq(t.as_bytes(), answer.as_bytes()))
        }
        _ => return Err(CoreError::CannotUnlockEarly),
    };
    if !correct {
        return Err(CoreError::WrongAnswer);
    }
    if let Some(at) = set.pending_unlock_at {
        return Ok(UnlockOutcome::Pending { at });
    }
    if lock.unlock_delay_secs == 0 {
        // Opened: the block turns off with it.
        set.lock = None;
        set.enabled = false;
        Ok(UnlockOutcome::Unlocked)
    } else {
        let at = now.unix + lock.unlock_delay_secs as i64;
        set.pending_unlock_at = Some(at);
        Ok(UnlockOutcome::Pending { at })
    }
}

/// Argon2id password hash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PasswordHash {
    pub salt: String,
    pub hash: String,
    pub m_kib: u32,
    pub t: u32,
    pub p: u32,
}

impl PasswordHash {
    /// OWASP-recommended Argon2id parameters.
    const M_KIB: u32 = 19 * 1024;
    const T: u32 = 2;
    const P: u32 = 1;

    pub fn new(password: &str) -> Result<Self, CoreError> {
        let mut salt = [0u8; 16];
        random_bytes(&mut salt)?;
        let hash = argon2id(password.as_bytes(), &salt, Self::M_KIB, Self::T, Self::P)?;
        Ok(Self { salt: to_hex(&salt), hash: to_hex(&hash), m_kib: Self::M_KIB, t: Self::T, p: Self::P })
    }

    /// Whether some password can match this: a real salt and hash, and
    /// parameters Argon2 accepts. The blank stand-in that non-administrators
    /// are shown is not.
    pub fn is_usable(&self) -> bool {
        from_hex(&self.salt).is_some_and(|s| s.len() >= 8)
            && from_hex(&self.hash).is_some_and(|h| h.len() == 32)
            && Params::new(self.m_kib, self.t, self.p, Some(32)).is_ok()
    }

    pub fn verify(&self, password: &str) -> bool {
        let (Some(salt), Some(expected)) = (from_hex(&self.salt), from_hex(&self.hash)) else {
            return false;
        };
        argon2id(password.as_bytes(), &salt, self.m_kib, self.t, self.p)
            .is_ok_and(|h| constant_time_eq(&h, &expected))
    }
}

fn argon2id(password: &[u8], salt: &[u8], m: u32, t: u32, p: u32) -> Result<[u8; 32], CoreError> {
    let params = Params::new(m, t, p, Some(32)).map_err(|_| CoreError::Hash)?;
    let mut out = [0u8; 32];
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password, salt, &mut out)
        .map_err(|_| CoreError::Hash)?;
    Ok(out)
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

pub(crate) fn random_bytes(buf: &mut [u8]) -> Result<(), CoreError> {
    getrandom::fill(buf).map_err(|_| CoreError::Random)
}

/// Characters that are easy to read and type: no 0/O, 1/l/I.
const TEXT_CHARSET: &[u8] = b"abcdefghijkmnopqrstuvwxyzABCDEFGHJKLMNPQRSTUVWXYZ23456789";

/// Random text for random-text locks, and passwords for a friend to keep.
pub fn random_text(len: usize) -> Result<String, CoreError> {
    let n = TEXT_CHARSET.len();
    let limit = 256 - 256 % n; // rejection sampling keeps it uniform
    let mut out = String::with_capacity(len);
    let mut buf = [0u8; 64];
    while out.len() < len {
        random_bytes(&mut buf)?;
        for &b in &buf {
            if (b as usize) < limit && out.len() < len {
                out.push(TEXT_CHARSET[b as usize % n] as char);
            }
        }
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Editing locked sets
// ---------------------------------------------------------------------------

impl Config {
    /// Adds a new set or replaces an existing one. A locked set may only be
    /// replaced by a version that is at least as strict.
    pub fn upsert_set(&mut self, new: BlockSet, ctx: &LockContext) -> Result<(), CoreError> {
        validate_set(&new)?;
        let mut new = new;
        match self.block_sets.iter_mut().find(|s| s.id == new.id) {
            Some(old) => {
                if is_locked(old, ctx)
                    && let Some(reason) = weakening_reason(old, &new)
                {
                    return Err(CoreError::Locked(reason));
                }
                // The same times lock keeps its last switch (so a block
                // switched on by hand outside its times stays on); new times
                // switch the block at once.
                new.times_on = if old.lock.as_ref().map(|l| &l.kind) == new.lock.as_ref().map(|l| &l.kind) {
                    old.times_on
                } else {
                    None
                };
                *old = new;
                apply_times(old, &ctx.now);
            }
            None => {
                new.times_on = None;
                apply_times(&mut new, &ctx.now);
                self.block_sets.push(new);
            }
        }
        Ok(())
    }

    pub fn any_locked(&self, ctx: &LockContext) -> bool {
        self.block_sets.iter().any(|s| is_locked(s, ctx))
    }

    /// Hardened mode can always be turned on, but off only while nothing
    /// is locked.
    pub fn set_hardened(&mut self, on: bool, ctx: &LockContext) -> Result<(), CoreError> {
        if self.hardened && !on && self.any_locked(ctx) {
            return Err(CoreError::Locked("turns off hardened mode"));
        }
        self.hardened = on;
        Ok(())
    }

    pub fn remove_set(&mut self, id: SetId, ctx: &LockContext) -> Result<BlockSet, CoreError> {
        let idx = self.block_sets.iter().position(|s| s.id == id).ok_or(CoreError::NotFound)?;
        if is_locked(&self.block_sets[idx], ctx) {
            return Err(CoreError::Locked("deletes the block"));
        }
        Ok(self.block_sets.remove(idx))
    }
}

pub(crate) fn validate_set(set: &BlockSet) -> Result<(), CoreError> {
    if let Some(s) = &set.schedule {
        s.validate()?;
    }
    if let Some(lock) = &set.lock {
        match &lock.kind {
            LockKind::Schedule { schedule } => schedule.validate()?,
            LockKind::Location { area } => area.validate()?,
            // A lock that could never be opened is refused.
            LockKind::Password { hash } if !hash.is_usable() => return Err(CoreError::Hash),
            _ => {}
        }
    }
    Ok(())
}

fn superset<T: Eq + Hash>(big: &[T], small: &[T]) -> bool {
    if small.len() <= 4 {
        return small.iter().all(|x| big.contains(x));
    }
    let big: HashSet<&T> = big.iter().collect();
    small.iter().all(|x| big.contains(x))
}

/// Why replacing `old` with `new` would make blocking weaker, if it would.
pub fn weakening_reason(old: &BlockSet, new: &BlockSet) -> Option<&'static str> {
    if old.id != new.id {
        return Some("a different block");
    }
    if old.enabled && !new.enabled {
        return Some("disables the block");
    }
    if !scope_not_narrower(&old.users, &new.users) {
        return Some("applies to fewer users");
    }

    let (ow, nw) = (&old.web, &new.web);
    if ow.mode != nw.mode {
        return Some("changes the website blocking mode");
    }
    if !superset(&nw.block, &ow.block) || !superset(&nw.lists, &ow.lists) {
        return Some("removes blocked websites");
    }
    if !superset(&ow.allow, &nw.allow) {
        return Some("adds allowed websites or exceptions");
    }
    if ow.safe_search && !nw.safe_search {
        return Some("turns off SafeSearch");
    }

    let (oa, na) = (&old.apps, &new.apps);
    if oa.mode != na.mode {
        return Some("changes the app blocking mode");
    }
    if !superset(&na.block, &oa.block) {
        return Some("removes blocked apps");
    }
    if !superset(&oa.allow, &na.allow) {
        return Some("adds allowed apps or exceptions");
    }

    match (&old.schedule, &new.schedule) {
        (_, None) => {}
        (None, Some(_)) => return Some("limits the block to a schedule"),
        (Some(o), Some(n)) if !schedule_covers(n, o) => return Some("shortens the schedule"),
        _ => {}
    }
    match (old.allowance, new.allowance) {
        (_, None) => {}
        (None, Some(_)) => return Some("adds a time allowance"),
        (Some(o), Some(n)) if n.daily_secs > o.daily_secs => return Some("increases the time allowance"),
        _ => {}
    }

    match (&old.lock, &new.lock) {
        (None, _) => {}
        (Some(_), None) => return Some("removes the lock"),
        (Some(o), Some(n)) => {
            if n.unlock_delay_secs < o.unlock_delay_secs {
                return Some("shortens the unlock delay");
            }
            if !lock_not_weaker(&o.kind, &n.kind) {
                return Some("weakens the lock");
            }
        }
    }
    match (old.pending_unlock_at, new.pending_unlock_at) {
        (_, None) => {}
        (Some(o), Some(n)) if n == o => {}
        _ => return Some("changes a pending unlock"),
    }
    None
}

fn scope_not_narrower(old: &UserScope, new: &UserScope) -> bool {
    use crate::model::eq_ignore_case as eq;
    let has = |list: &[String], name: &String| list.iter().any(|n| eq(n, name));
    match (old, new) {
        (_, UserScope::All) => true,
        (UserScope::All, _) => false,
        (UserScope::AllExcept(o), UserScope::AllExcept(n)) => n.iter().all(|x| has(o, x)),
        (UserScope::AllExcept(_), UserScope::Only(_)) => false,
        (UserScope::Only(o), UserScope::Only(n)) => o.iter().all(|x| has(n, x)),
        (UserScope::Only(o), UserScope::AllExcept(n)) => !o.iter().any(|x| has(n, x)),
    }
}

/// Does `big` contain every window of `small`?
fn schedule_covers(big: &Schedule, small: &Schedule) -> bool {
    small.weekly.iter().all(|w| big.weekly.contains(w)) && small.dates.iter().all(|d| big.dates.contains(d))
}

fn lock_not_weaker(old: &LockKind, new: &LockKind) -> bool {
    match (old, new) {
        (LockKind::Until { until: o }, LockKind::Until { until: n }) => n >= o,
        (LockKind::Schedule { schedule: o }, LockKind::Schedule { schedule: n }) => schedule_covers(n, o),
        // A new password would let the user pick one they know.
        (LockKind::Password { hash: o }, LockKind::Password { hash: n }) => o == n,
        (LockKind::RandomText { length: o }, LockKind::RandomText { length: n }) => n >= o,
        (LockKind::UntilRestart { boot_id: o }, LockKind::UntilRestart { boot_id: n }) => o == n,
        (LockKind::Location { area: o }, LockKind::Location { area: n }) => o == n,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::GeoArea;
    use crate::model::{Allowance, ListMode};
    use crate::schedule::{Day, WeeklyRange};
    use crate::web::WebRule;
    use jiff::civil::date;

    fn now(unix: i64) -> Now {
        Now { unix, local: date(2026, 10, 12).at(10, 0, 0, 0) }
    }

    fn ctx(unix: i64) -> LockContext<'static> {
        LockContext { now: now(unix), boot_id: "boot-1", location: LocationFix::Unknown, inside: &[], outside: &[] }
    }

    fn locked_set(kind: LockKind) -> BlockSet {
        let mut s = BlockSet::new("test");
        s.web.block = vec![WebRule::parse("example.com").unwrap()];
        s.lock = Some(Lock { kind, unlock_delay_secs: 0 });
        s
    }

    #[test]
    fn until_lock() {
        let mut s = locked_set(LockKind::Until { until: 100 });
        assert!(is_locked(&s, &ctx(99)));
        assert!(!is_locked(&s, &ctx(100)));
        assert!(!expire(&mut s, &ctx(99)));
        assert!(expire(&mut s, &ctx(100)));
        assert!(s.lock.is_none());
        assert!(!s.enabled, "the block ends with its timer");
    }

    #[test]
    fn times_lock_switches_the_block() {
        // Mondays 09:00-12:00; ctx() is Monday 10:00.
        let times = Schedule { weekly: vec![WeeklyRange { days: vec![Day::Mon], start: 9 * 60, end: 12 * 60 }], dates: vec![] };
        let mut s = locked_set(LockKind::Schedule { schedule: times });
        s.enabled = false;
        let at = |h: i8| LockContext { now: Now { unix: 0, local: date(2026, 10, 12).at(h, 0, 0, 0) }, ..ctx(0) };

        assert!(expire(&mut s, &at(10)), "the times started");
        assert!(s.enabled && is_locked(&s, &at(10)));
        assert!(!expire(&mut s, &at(11)), "nothing new");
        assert!(expire(&mut s, &at(13)), "the times ended");
        assert!(!s.enabled && !is_locked(&s, &at(13)));
        // Switched on by hand outside the times: it stays on until they end.
        s.enabled = true;
        assert!(!expire(&mut s, &at(14)));
        assert!(s.enabled);

        // Saved with this lock outside its times: off at once.
        let mut config = Config::default();
        let mut new = locked_set(s.lock.clone().unwrap().kind);
        new.enabled = true;
        config.upsert_set(new, &at(14)).unwrap();
        assert!(!config.block_sets[0].enabled);
    }

    #[test]
    fn restart_lock() {
        let mut s = locked_set(LockKind::UntilRestart { boot_id: "boot-1".into() });
        assert!(is_locked(&s, &ctx(0)));
        let after_reboot = LockContext { boot_id: "boot-2", ..ctx(0) };
        assert!(!is_locked(&s, &after_reboot));
        assert!(expire(&mut s, &after_reboot));
        assert!(s.enabled, "a restart lock only ends the lock");
    }

    #[test]
    fn location_lock_fails_closed() {
        let area = GeoArea::Circle { lat: 0.0, lon: 0.0, radius_m: 100.0 };
        let s = locked_set(LockKind::Location { area });
        assert!(is_locked(&s, &ctx(0)), "unknown location counts as inside");
        let far = LockContext { location: LocationFix::Known { lat: 1.0, lon: 1.0, accuracy_m: 50.0 }, ..ctx(0) };
        assert!(!is_locked(&s, &far));
        let areas = [GeoArea::Circle { lat: 0.0, lon: 0.0, radius_m: 100.0 }];
        let wifi_says_inside = LockContext { inside: &areas, ..far };
        assert!(is_locked(&s, &wifi_says_inside), "learned Wi-Fi beats a remote fix");
        let wifi_says_outside = LockContext { outside: &areas, ..ctx(0) };
        assert!(!is_locked(&s, &wifi_says_outside), "away from the learned Wi-Fi");
    }

    #[test]
    fn schedule_lock() {
        let schedule = Schedule {
            weekly: vec![WeeklyRange { days: vec![Day::Mon], start: 9 * 60, end: 17 * 60 }],
            dates: vec![],
        };
        let s = locked_set(LockKind::Schedule { schedule });
        assert!(is_locked(&s, &ctx(0)), "Monday 10:00");
    }

    #[test]
    fn password_unlock() {
        let hash = PasswordHash::new("correct horse").unwrap();
        let mut s = locked_set(LockKind::Password { hash });
        assert_eq!(challenge_for(s.lock.as_ref().unwrap()).unwrap(), Challenge::Password);
        assert!(matches!(try_unlock(&mut s, "wrong", None, &now(0)), Err(CoreError::WrongAnswer)));
        assert!(s.lock.is_some());
        assert!(s.enabled);
        assert_eq!(try_unlock(&mut s, "correct horse", None, &now(0)).unwrap(), UnlockOutcome::Unlocked);
        assert!(s.lock.is_none());
        assert!(!s.enabled, "an opened lock turns its block off");
    }

    #[test]
    fn unusable_passwords_are_refused_and_removed() {
        let blank = PasswordHash { salt: String::new(), hash: String::new(), m_kib: 0, t: 0, p: 0 };
        assert!(!blank.is_usable());
        assert!(PasswordHash::new("x").unwrap().is_usable());
        let mut s = locked_set(LockKind::Password { hash: blank });
        assert!(matches!(validate_set(&s), Err(CoreError::Hash)));
        // One already stored (by an older Iqal) can never be opened: removed.
        assert!(expire(&mut s, &ctx(0)));
        assert!(s.lock.is_none());
        assert!(!s.enabled, "removed like an opened lock: the block turns off");
    }

    #[test]
    fn random_text_unlock_with_delay() {
        let mut s = locked_set(LockKind::RandomText { length: 40 });
        s.lock.as_mut().unwrap().unlock_delay_secs = 600;
        let Challenge::RandomText(text) = challenge_for(s.lock.as_ref().unwrap()).unwrap() else { panic!() };
        assert_eq!(text.len(), 40);
        assert!(matches!(try_unlock(&mut s, &text, None, &now(0)), Err(CoreError::WrongAnswer)));
        assert!(matches!(try_unlock(&mut s, "nope", Some(&text), &now(0)), Err(CoreError::WrongAnswer)));
        assert_eq!(try_unlock(&mut s, &text, Some(&text), &now(1000)).unwrap(), UnlockOutcome::Pending { at: 1600 });
        assert!(is_locked(&s, &ctx(1500)), "still locked during the delay");
        assert!(!expire(&mut s, &ctx(1599)));
        assert!(s.enabled, "on during the delay");
        assert!(expire(&mut s, &ctx(1600)));
        assert!(!is_locked(&s, &ctx(1600)));
        assert!(!s.enabled, "off once the delayed unlock happens");
    }

    #[test]
    fn time_locks_cannot_be_unlocked_early() {
        let mut s = locked_set(LockKind::Until { until: i64::MAX });
        assert_eq!(challenge_for(s.lock.as_ref().unwrap()).unwrap(), Challenge::NotPossible);
        assert!(matches!(try_unlock(&mut s, "x", None, &now(0)), Err(CoreError::CannotUnlockEarly)));
    }

    #[test]
    fn random_text_is_varied() {
        let a = random_text(32).unwrap();
        let b = random_text(32).unwrap();
        assert_ne!(a, b);
        assert!(a.bytes().all(|c| TEXT_CHARSET.contains(&c)));
    }

    #[test]
    fn locked_edits_must_be_stricter() {
        let ctx = ctx(0);
        let mut config = Config::default();
        let set = locked_set(LockKind::Until { until: 1_000 });
        let id = set.id;
        config.upsert_set(set.clone(), &ctx).unwrap();

        // Stricter: more sites, SafeSearch on, a later deadline.
        let mut stricter = set.clone();
        stricter.web.block.push(WebRule::parse("reddit.com").unwrap());
        stricter.web.safe_search = true;
        stricter.lock = Some(Lock { kind: LockKind::Until { until: 2_000 }, unlock_delay_secs: 0 });
        config.upsert_set(stricter.clone(), &ctx).unwrap();

        let weaker = |f: &dyn Fn(&mut BlockSet)| {
            let mut s = stricter.clone();
            f(&mut s);
            s
        };
        let attempts: Vec<BlockSet> = vec![
            weaker(&|s| s.web.block.clear()),
            weaker(&|s| s.web.allow.push(WebRule::parse("example.com/ok").unwrap())),
            weaker(&|s| s.web.safe_search = false),
            weaker(&|s| s.web.mode = ListMode::AllowOnly),
            weaker(&|s| s.enabled = false),
            weaker(&|s| s.users = UserScope::Only(vec!["bob".into()])),
            weaker(&|s| s.lock = None),
            weaker(&|s| s.lock = Some(Lock { kind: LockKind::Until { until: 1_500 }, unlock_delay_secs: 0 })),
            weaker(&|s| s.allowance = Some(Allowance { daily_secs: 60 })),
            weaker(&|s| s.schedule = Some(Schedule::default())),
        ];
        for attempt in attempts {
            assert!(matches!(config.upsert_set(attempt, &ctx), Err(CoreError::Locked(_))));
        }
        assert!(matches!(config.remove_set(id, &ctx), Err(CoreError::Locked(_))));

        // Once the lock has passed, anything goes.
        let later = super::tests::ctx(5_000);
        let mut free = stricter.clone();
        free.web.block.clear();
        config.upsert_set(free, &later).unwrap();
        config.remove_set(id, &later).unwrap();
        assert!(config.block_sets.is_empty());
    }

    #[test]
    fn hardened_mode_stays_on_while_locked() {
        let mut config = Config::default();
        config.block_sets.push(locked_set(LockKind::Until { until: 100 }));
        config.set_hardened(true, &ctx(0)).unwrap();
        assert!(matches!(config.set_hardened(false, &ctx(50)), Err(CoreError::Locked(_))));
        config.set_hardened(false, &ctx(100)).unwrap();
        assert!(!config.hardened);
    }

    #[test]
    fn user_scope_narrowing() {
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        use UserScope::*;
        assert!(scope_not_narrower(&Only(s(&["a"])), &Only(s(&["a", "b"]))));
        assert!(!scope_not_narrower(&Only(s(&["a", "b"])), &Only(s(&["a"]))));
        assert!(scope_not_narrower(&AllExcept(s(&["a", "b"])), &AllExcept(s(&["A"]))));
        assert!(!scope_not_narrower(&AllExcept(s(&["a"])), &AllExcept(s(&["a", "b"]))));
        assert!(scope_not_narrower(&Only(s(&["a"])), &AllExcept(s(&["b"]))));
        assert!(!scope_not_narrower(&Only(s(&["a"])), &AllExcept(s(&["a"]))));
        assert!(scope_not_narrower(&AllExcept(s(&["a"])), &All));
        assert!(!scope_not_narrower(&All, &AllExcept(s(&["a"]))));
    }

    #[test]
    fn password_change_is_weakening() {
        let o = LockKind::Password { hash: PasswordHash::new("a").unwrap() };
        let n = LockKind::Password { hash: PasswordHash::new("a").unwrap() };
        assert!(!lock_not_weaker(&o, &n), "different salt = a password the user just chose");
        assert!(lock_not_weaker(&o, &o.clone()));
    }
}
