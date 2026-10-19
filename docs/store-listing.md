# Store listings for the Iqal extension

What to enter in the Chrome Web Store and Microsoft Edge Add-ons forms.
Files to upload are in `dist/store/` (made by `cargo run -p iqal-pack`, then copied):

| File | Use |
|---|---|
| `iqal-store.zip` | The extension package (both stores) |
| `icon-128.png` | Store icon, 128×128 (Chrome) |
| `logo-300.png` | Store logo, 300×300 (Edge) |
| `screenshot-en.png`, `screenshot-ar.png` | Screenshots, 1280×800 (both stores) |

## Shared texts

**Name:** Iqal (taken from the package)

**Short description** (taken from the package):
Website blocking for the Iqal blocker. Managed by Iqal; cannot be removed while Iqal is installed.

**Detailed description:**

> Iqal is a free, open-source website and app blocker for Windows and Linux. This extension is its browser part: it checks each page you open against the blocks you set up in the Iqal app, and shows a block page when one applies — including rules that match part of an address or a search term, which network-level blocking alone cannot see.
>
> The extension only works together with the Iqal app installed on the same computer, which installs it and keeps it in place. On its own it does nothing.
>
> Privacy: page addresses are checked by the Iqal app on your own computer. Nothing is sent to the developer or to any other server.
>
> Available in English, Arabic, French, Spanish, German, Turkish, Urdu, Indonesian, Portuguese, Russian and Chinese.

**Category:** Productivity

**Language:** English

**Privacy policy URL:** (where `docs/privacy-policy.md` is published)

## Chrome Web Store — Privacy practices tab

**Single purpose:**
Blocks web pages that the user has chosen to block in the Iqal app on the same computer.

**Permission justifications:**

| Permission | Justification |
|---|---|
| nativeMessaging | Talks to the Iqal app on the same computer, which holds the user's block rules and decides whether a page is blocked. |
| tabs | Reads the address of open tabs to check them again when rules change (for example when a scheduled block starts) and to replace a blocked page with the block page. Also tells the Iqal app which page is in front, for daily time allowances the user set. |
| webNavigation | Checks each page while it is being opened, so a blocked page is stopped before it loads. |

**Are you using remote code?** No.

**Data usage:** tick **Web history** (page addresses are checked; they never leave the computer). Leave everything else unticked.
Then tick all three certifications (not sold to third parties; not used for unrelated purposes; not used for creditworthiness or lending).

## Chrome Web Store — Distribution tab

- **Visibility:** Unlisted
- **Regions:** All regions

## Microsoft Edge Add-ons

- **Visibility:** Hidden
- **Markets:** All markets
- **Category:** Productivity
- **Privacy policy:** Yes — the same URL
- **Notes for certification:**

> This extension is the browser part of Iqal, a desktop website/app blocker. It connects through native messaging to the host "org.iqal.host", which is installed by the Iqal app. Without the app it stays idle and never blocks anything. Page addresses are only passed to the local app; nothing is sent over the network.

## After publishing

Each store gives the extension its own id (32 letters). Iqal needs both ids to register its native host for them and to install the extension in Chrome, Brave and Edge.
