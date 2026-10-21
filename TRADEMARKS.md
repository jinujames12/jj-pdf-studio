# Trademarks and branding

The source code of JJ PDF Studio is licensed under the [Apache License 2.0](LICENSE). That
license, like all Apache-2.0 licenses, **does not grant any rights to use trademarks, service
marks or product names** (section 6).

"JJ PDF Studio", "James & James LLP" and the JJ logo are the property of James & James LLP. The logo and other
files in [`assets/brand/`](assets/brand/) are all rights reserved and are not covered by the
project license.

## What you may do

- Fork, modify and redistribute the code under the terms of Apache-2.0.
- Say that your project is "based on JJ PDF Studio" or "a fork of JJ PDF Studio", as long as it
  is clear that it is not the official version.

## What you may not do

- Distribute a modified or unmodified build under the name "JJ PDF Studio", or a name likely to
  be confused with it.
- Use the JJ logo or any `assets/brand/` file in your fork, app icon, website or marketing.
- Suggest that James & James LLP made, endorsed or supports your fork.

## Forking checklist

1. Choose a new product name and identifier (`productName`, `identifier` and `mainBinaryName` in
   `src-tauri/tauri.conf.json`).
2. Replace `assets/brand/logo.svg` and `logo.png`, then run `npm run tauri icon assets/brand/logo.png`.
3. Update the About screen, README and window title.
4. Keep the `LICENSE` file and copyright notices for the code you reuse.

Questions about permission to use the brand: TODO add a contact address.
