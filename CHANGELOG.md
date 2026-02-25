# Changelog

## [0.7.0](https://github.com/denolehov/annot/compare/v0.6.0...v0.7.0) (2026-02-25)


### Features

* add Homebrew tap automation to release workflow ([1605ef0](https://github.com/denolehov/annot/commit/1605ef02ea0c96f3c5698300f8f22b947005a01e))

## [0.6.0](https://github.com/denolehov/annot/compare/v0.5.0...v0.6.0) (2026-02-25)


### Features

* add ? help overlay ([7a5e27c](https://github.com/denolehov/annot/commit/7a5e27cd60ac376091703756edceeb3c053adea3))
* add /remove command that is like /replace but with empty content ([493a829](https://github.com/denolehov/annot/commit/493a829bf84379e7e416ac143e470437a83c44a5))
* add an ability to copy tables ([5afd422](https://github.com/denolehov/annot/commit/5afd4223e0873be2624f20cba9f917a877422a1d))
* add HEX color preview ([7d87463](https://github.com/denolehov/annot/commit/7d87463456bcce7bd5aaaf15bf869ffd2f71ea77))
* add icons for each of the terraform axis ([d60a402](https://github.com/denolehov/annot/commit/d60a4026f1112d6f97affd3694a341391185c8e8))
* add sorting to `annot bookmarks list` command ([3e16168](https://github.com/denolehov/annot/commit/3e16168c4f9d31515dc8a4628d783668ca63be6a))
* add svg icons to distinguish different item types in @ ref menu ([2f73984](https://github.com/denolehov/annot/commit/2f73984319fa4bbb303deac90f27c0434c5235f2))
* add terraform axes precedence rules ([52654f8](https://github.com/denolehov/annot/commit/52654f8685b21bae080453269b6444e5dd0419cf))
* add terraform regions for structured content transformation ([bb9c082](https://github.com/denolehov/annot/commit/bb9c0826091dbd427685dd9469acecb0af62f0ec))
* add version subcommand ([62d6b74](https://github.com/denolehov/annot/commit/62d6b7428a05f369bf255f5efc953908c580cd0c))
* allow referencing markdown headings via @ in annotation ([9eed904](https://github.com/denolehov/annot/commit/9eed9043ea7fdae9b64ceff520d7dc6971b7106b))
* CLI --json output, --exit-mode flag, and structured metadata ([16d189d](https://github.com/denolehov/annot/commit/16d189daf199df9351ba652287ef35a91286e35b))
* natural prose terraform directives ([6ce5c0c](https://github.com/denolehov/annot/commit/6ce5c0c09b22dd7ffd8f513c16ab644bacde2516))
* remove redundant categories from the @ ref menu so that fuzzy search works properly ([d83657b](https://github.com/denolehov/annot/commit/d83657bf421e15a52c0e72fad79a54584ffaec3c))
* show toast on successful terraform apply ([45b30fa](https://github.com/denolehov/annot/commit/45b30fa8d3f086fa4884c31688fbd665e8d92eb3))
* the bookmarks subcommand is now `annot bookmark` (singular) ([5b4f7ae](https://github.com/denolehov/annot/commit/5b4f7ae23aacf93f3c85711d78b2bd047b5060af))
* use fuzzy search in @ annotation menu ([8f35456](https://github.com/denolehov/annot/commit/8f354560ef056a87319998d81f89cad6bfb92c12))


### Bug Fixes

* ? help modal layout organization ([4db4b28](https://github.com/denolehov/annot/commit/4db4b2830a54cc6003512299179c068705a5c6d7))
* @ ref menu ranking improvements ([7cd2e48](https://github.com/denolehov/annot/commit/7cd2e48479fb99137087e0c8906a66b4cdea8c62))
* allow blurring the annotation editor if tag/ref/command suggestion menu is active ([e76a1ba](https://github.com/denolehov/annot/commit/e76a1ba9eb6b0ba03dcc6b125b491fb5eb1245f4))
* allow opening session editor from hovering phase ([af5a317](https://github.com/denolehov/annot/commit/af5a3173aa838ab2f19f03b3f512c89c97efa4d8))
* build workflow now accepts tag param ([8ca04ad](https://github.com/denolehov/annot/commit/8ca04ad3b2c49e1dc8b85d1a65088b08fe20192b))
* chip font size is now consistent with the annotation editor size ([3a2ec96](https://github.com/denolehov/annot/commit/3a2ec96c8165e91859a98f3d1bb82db1a02b6197))
* contradictory terraform prose outputs are no longer possible ([89017b6](https://github.com/denolehov/annot/commit/89017b6281cada5dd74179bf5afa484555c57de8))
* fix buggy excalidraw close confirmation dialog ([004dc32](https://github.com/denolehov/annot/commit/004dc324ec2df5ccb01680ab154ab3e6d2767614))
* hunk context is now properly tracked in diff mode and shown in ([dae79bb](https://github.com/denolehov/annot/commit/dae79bb48613b0c21247bb7db01442dd083cb716))
* make non-sensical terraform states impossible to represent ([bf3aa5a](https://github.com/denolehov/annot/commit/bf3aa5a55432081b5b11a7c8714ead08f3e8c56d))
* prevent orphaned excalidraw windows if excalidraw chip was deleted ([d9d5e3e](https://github.com/denolehov/annot/commit/d9d5e3eda092e829b08ea3b83a78b328bc1810c5))
* properly track open editor to prevent double open editors ([ee8a06b](https://github.com/denolehov/annot/commit/ee8a06b3a2e57f1dd59a5737dec1b324fb5cfd2c))
* reset selected text when selecting for annotation ([0f3b567](https://github.com/denolehov/annot/commit/0f3b56741c5f8bda481e1e79ecbae6505a7a8f54))
* sealed annotation editors no longer disappear when selecting new ([d0c63cd](https://github.com/denolehov/annot/commit/d0c63cde5a4531ad1569a9d34898013c23426165))
* terraform element padding is now consistent with the annotation editor padding ([46f8a09](https://github.com/denolehov/annot/commit/46f8a091b3dbad37cfef419b072263ce7b9e6079))
* treat kotlin syntax as java for highlighting ([646fbec](https://github.com/denolehov/annot/commit/646fbecb67a5ac34ccfdd30ca927d43cf17831bd))
* trigger build workflow on release publish event ([45e064b](https://github.com/denolehov/annot/commit/45e064b85027b9f612640f803d3c2bd2f50cdc7b))
* use rangeKey instead of interaction.range in updateAnnotation ([416d536](https://github.com/denolehov/annot/commit/416d536e6abd2d74c23fe87f81a5e85753145ac9))
* yaml list items are no longer broken ([72a4ebb](https://github.com/denolehov/annot/commit/72a4ebbab20955fdcbeea89fa4139477ae784bc0))

## [0.5.0](https://github.com/denolehov/annot/compare/v0.4.0...v0.5.0) (2026-02-25)


### Features

* add ? help overlay ([7a5e27c](https://github.com/denolehov/annot/commit/7a5e27cd60ac376091703756edceeb3c053adea3))
* add /remove command that is like /replace but with empty content ([493a829](https://github.com/denolehov/annot/commit/493a829bf84379e7e416ac143e470437a83c44a5))
* add an ability to copy tables ([5afd422](https://github.com/denolehov/annot/commit/5afd4223e0873be2624f20cba9f917a877422a1d))
* add HEX color preview ([7d87463](https://github.com/denolehov/annot/commit/7d87463456bcce7bd5aaaf15bf869ffd2f71ea77))
* add icons for each of the terraform axis ([d60a402](https://github.com/denolehov/annot/commit/d60a4026f1112d6f97affd3694a341391185c8e8))
* add sorting to `annot bookmarks list` command ([3e16168](https://github.com/denolehov/annot/commit/3e16168c4f9d31515dc8a4628d783668ca63be6a))
* add svg icons to distinguish different item types in @ ref menu ([2f73984](https://github.com/denolehov/annot/commit/2f73984319fa4bbb303deac90f27c0434c5235f2))
* add terraform axes precedence rules ([52654f8](https://github.com/denolehov/annot/commit/52654f8685b21bae080453269b6444e5dd0419cf))
* add terraform regions for structured content transformation ([bb9c082](https://github.com/denolehov/annot/commit/bb9c0826091dbd427685dd9469acecb0af62f0ec))
* add version subcommand ([62d6b74](https://github.com/denolehov/annot/commit/62d6b7428a05f369bf255f5efc953908c580cd0c))
* allow referencing markdown headings via @ in annotation ([9eed904](https://github.com/denolehov/annot/commit/9eed9043ea7fdae9b64ceff520d7dc6971b7106b))
* CLI --json output, --exit-mode flag, and structured metadata ([16d189d](https://github.com/denolehov/annot/commit/16d189daf199df9351ba652287ef35a91286e35b))
* natural prose terraform directives ([6ce5c0c](https://github.com/denolehov/annot/commit/6ce5c0c09b22dd7ffd8f513c16ab644bacde2516))
* remove redundant categories from the @ ref menu so that fuzzy search works properly ([d83657b](https://github.com/denolehov/annot/commit/d83657bf421e15a52c0e72fad79a54584ffaec3c))
* show toast on successful terraform apply ([45b30fa](https://github.com/denolehov/annot/commit/45b30fa8d3f086fa4884c31688fbd665e8d92eb3))
* the bookmarks subcommand is now `annot bookmark` (singular) ([5b4f7ae](https://github.com/denolehov/annot/commit/5b4f7ae23aacf93f3c85711d78b2bd047b5060af))
* use fuzzy search in @ annotation menu ([8f35456](https://github.com/denolehov/annot/commit/8f354560ef056a87319998d81f89cad6bfb92c12))


### Bug Fixes

* ? help modal layout organization ([4db4b28](https://github.com/denolehov/annot/commit/4db4b2830a54cc6003512299179c068705a5c6d7))
* @ ref menu ranking improvements ([7cd2e48](https://github.com/denolehov/annot/commit/7cd2e48479fb99137087e0c8906a66b4cdea8c62))
* allow blurring the annotation editor if tag/ref/command suggestion menu is active ([e76a1ba](https://github.com/denolehov/annot/commit/e76a1ba9eb6b0ba03dcc6b125b491fb5eb1245f4))
* allow opening session editor from hovering phase ([af5a317](https://github.com/denolehov/annot/commit/af5a3173aa838ab2f19f03b3f512c89c97efa4d8))
* build workflow now accepts tag param ([8ca04ad](https://github.com/denolehov/annot/commit/8ca04ad3b2c49e1dc8b85d1a65088b08fe20192b))
* chip font size is now consistent with the annotation editor size ([3a2ec96](https://github.com/denolehov/annot/commit/3a2ec96c8165e91859a98f3d1bb82db1a02b6197))
* contradictory terraform prose outputs are no longer possible ([89017b6](https://github.com/denolehov/annot/commit/89017b6281cada5dd74179bf5afa484555c57de8))
* fix buggy excalidraw close confirmation dialog ([004dc32](https://github.com/denolehov/annot/commit/004dc324ec2df5ccb01680ab154ab3e6d2767614))
* hunk context is now properly tracked in diff mode and shown in ([dae79bb](https://github.com/denolehov/annot/commit/dae79bb48613b0c21247bb7db01442dd083cb716))
* make non-sensical terraform states impossible to represent ([bf3aa5a](https://github.com/denolehov/annot/commit/bf3aa5a55432081b5b11a7c8714ead08f3e8c56d))
* prevent orphaned excalidraw windows if excalidraw chip was deleted ([d9d5e3e](https://github.com/denolehov/annot/commit/d9d5e3eda092e829b08ea3b83a78b328bc1810c5))
* properly track open editor to prevent double open editors ([ee8a06b](https://github.com/denolehov/annot/commit/ee8a06b3a2e57f1dd59a5737dec1b324fb5cfd2c))
* reset selected text when selecting for annotation ([0f3b567](https://github.com/denolehov/annot/commit/0f3b56741c5f8bda481e1e79ecbae6505a7a8f54))
* sealed annotation editors no longer disappear when selecting new ([d0c63cd](https://github.com/denolehov/annot/commit/d0c63cde5a4531ad1569a9d34898013c23426165))
* terraform element padding is now consistent with the annotation editor padding ([46f8a09](https://github.com/denolehov/annot/commit/46f8a091b3dbad37cfef419b072263ce7b9e6079))
* treat kotlin syntax as java for highlighting ([646fbec](https://github.com/denolehov/annot/commit/646fbecb67a5ac34ccfdd30ca927d43cf17831bd))
* trigger build workflow on release publish event ([45e064b](https://github.com/denolehov/annot/commit/45e064b85027b9f612640f803d3c2bd2f50cdc7b))
* use rangeKey instead of interaction.range in updateAnnotation ([416d536](https://github.com/denolehov/annot/commit/416d536e6abd2d74c23fe87f81a5e85753145ac9))
* yaml list items are no longer broken ([72a4ebb](https://github.com/denolehov/annot/commit/72a4ebbab20955fdcbeea89fa4139477ae784bc0))

## [0.4.0](https://github.com/denolehov/annot/compare/v0.3.0...v0.4.0) (2026-01-03)


### Features

* add ? help overlay ([988a479](https://github.com/denolehov/annot/commit/988a479cd50cfd7b27acf7ba4231b2b7eb15794f))


### Bug Fixes

* ? help modal layout organization ([f9ba157](https://github.com/denolehov/annot/commit/f9ba157e7bccf0b07cdee43e27a8e8c14fc47b53))
* reset selected text when selecting for annotation ([ce8c4c0](https://github.com/denolehov/annot/commit/ce8c4c073b32ef7e262115ce1f492fc188405151))

## [0.3.0](https://github.com/denolehov/annot/compare/v0.2.3...v0.3.0) (2026-01-03)


### Features

* add /remove command that is like /replace but with empty content ([414c8d9](https://github.com/denolehov/annot/commit/414c8d9c76285f459ca3c3b3851951346554a9d8))


### Bug Fixes

* chip font size is now consistent with the annotation editor size ([44e314b](https://github.com/denolehov/annot/commit/44e314b77840bc96b2ebfb82fa9e127f824f96bc))

## [0.2.3](https://github.com/denolehov/annot/compare/v0.2.2...v0.2.3) (2026-01-03)


### Bug Fixes

* sealed annotation editors no longer disappear when selecting new ([d8b0c37](https://github.com/denolehov/annot/commit/d8b0c37e1aa23c192af43c6009161ab4e3828c69))
* use rangeKey instead of interaction.range in updateAnnotation ([efbf573](https://github.com/denolehov/annot/commit/efbf573778264e69ce250c76019da2709ce1634a))

## [0.2.2](https://github.com/denolehov/annot/compare/v0.2.1...v0.2.2) (2026-01-03)


### Bug Fixes

* build workflow now accepts tag param ([e7af1f8](https://github.com/denolehov/annot/commit/e7af1f8aa7737d2edb23dc80aa14403658501c54))

## [0.2.1](https://github.com/denolehov/annot/compare/v0.2.0...v0.2.1) (2026-01-03)


### Bug Fixes

* trigger build workflow on release publish event ([db6e54a](https://github.com/denolehov/annot/commit/db6e54a81ee89c5d3012df7735f2e419f0f638f9))

## [0.2.0](https://github.com/denolehov/annot/compare/v0.1.0...v0.2.0) (2026-01-03)


### Features

* add an ability to reference bookmarks in annotations ([2d662ee](https://github.com/denolehov/annot/commit/2d662eea6434e0a7a71ed13d21f1e7ada3c53d38))
* add an ability to search the doc with Cmd-F ([a0a1a71](https://github.com/denolehov/annot/commit/a0a1a717bffe0f4c4eaa5f89b8a8a7192556dae7))
* add command palette kbd hint to footer ([8b20273](https://github.com/denolehov/annot/commit/8b202737db13f8aa235b34e675b62197b8cff74a))
* add context infrastructure for state management ([a38ce30](https://github.com/denolehov/annot/commit/a38ce30a6333a2642e2198c4036bf0e7514a30af))
* add project to bookmark list display in command palette ([7a0255f](https://github.com/denolehov/annot/commit/7a0255fe40b9cec1bc7d9cff840333b7ada43d3f))
* add zoom level indicator ([72422d2](https://github.com/denolehov/annot/commit/72422d2e032a25d85ee2ba9420ac023b66fc7c93))
* allow bookmarking line ranges ([fef5f37](https://github.com/denolehov/annot/commit/fef5f37d2d3441bb013816e9c7e170027a9f77ab))
* allow copying markdown sections ([2feb8d7](https://github.com/denolehov/annot/commit/2feb8d7326cb3a7721dcd7e122d8d5b3d60a2dd0))
* allow editing last-created bookmark with [e] ([09caae1](https://github.com/denolehov/annot/commit/09caae1821a9a79d2278597023c86b2836e8e560))
* allow managing session-level bookmarks ([0bf205e](https://github.com/denolehov/annot/commit/0bf205e23fce432965d15f8c2a53d09a4c40c619))
* allow opening bookmarks in annot ([7a9b9eb](https://github.com/denolehov/annot/commit/7a9b9eb6b7d516df6130c064fd4d96ef515f7421))
* allow referencing annotations in annotations ([a4c9fae](https://github.com/denolehov/annot/commit/a4c9faea81fde1f910ac0a9d6e86228d88099eb4))
* allow referencing files with @ in annotations ([7c4944c](https://github.com/denolehov/annot/commit/7c4944cee4c2c2dee03ded4bd22c14d4172cfea8))
* allow shift-drag &gt; annotate or shift-drag &gt; bookmark ([ddcc7bb](https://github.com/denolehov/annot/commit/ddcc7bbccd53d59680ca95454d1507fcce6bb1f7))
* allow sorting bookmarks by creation date in CLI ([fbe1557](https://github.com/denolehov/annot/commit/fbe15574299dcab4ef12ef5365468ab6f49f9936))
* allow using Claude Code commands as exit modes ([1df1f9a](https://github.com/denolehov/annot/commit/1df1f9a55478b42ed440e15425ac1d907857d255))
* alt-tab and cmd-enter to enter exit modes in CP and set them ([444494f](https://github.com/denolehov/annot/commit/444494fdaa6f843f87d3209f5ecae3d21b731878))
* auto-derive bookmark labels from markdown headings ([f9f597d](https://github.com/denolehov/annot/commit/f9f597dc798c4153a43850163411cb6373f057cb))
* auto-save excalidraw diagrams on cmd-w ([e9f2821](https://github.com/denolehov/annot/commit/e9f28217d279ea86cd5c5251dc7d81e2dbafa19e))
* bind f key to Fit action in Mermaid window ([c317418](https://github.com/denolehov/annot/commit/c317418f92875e85fc48af4bfd830e94b9e3e378))
* bookmark CLI and MCP tools ([b627dd2](https://github.com/denolehov/annot/commit/b627dd218f0f7ed1ce36ff3a120c34e1f6fd2f46))
* copy code buttons on code blocks ([0769e06](https://github.com/denolehov/annot/commit/0769e06c5fe61160c9c69da9da8490220f56db10))
* dark theme support ([a9c9e5e](https://github.com/denolehov/annot/commit/a9c9e5ece57aa920658c1881905af8314cedd81f))
* delete items in command palette with Cmd-D ([9a95c62](https://github.com/denolehov/annot/commit/9a95c6294eadee4cba72a9baf4242d3ee1e294b0))
* emit a note about saved files to output (if they were saved) ([aae5078](https://github.com/denolehov/annot/commit/aae5078c77c971120259e2bb713dcf5b8ec6659b))
* get rid of the emojis from the slash menu in the annotation editor ([d9b8095](https://github.com/denolehov/annot/commit/d9b80954ad9f0ee0e6cb67f8e63c53577c09700d))
* highlight individual words in /replace command ([5076652](https://github.com/denolehov/annot/commit/507665258afef391ee0b2470cdf9611217549c7e))
* mermaid to excalidraw diagarm to annotation ([f01416a](https://github.com/denolehov/annot/commit/f01416a0530ef57084d8818ced1597c3499a7ef0))
* migrate AnnotationSlot to use context (11 props -&gt; 6 props) ([e4b2b5a](https://github.com/denolehov/annot/commit/e4b2b5a9800c3e6dc790d26e92ed0e0e1a448102))
* migrate Header to use context (metadata, showToast) ([6d0400a](https://github.com/denolehov/annot/commit/6d0400a8f67b64e46576f212c0185facf4f65fa9))
* migrate Portal, CodeBlock, Table to use context ([1a7182f](https://github.com/denolehov/annot/commit/1a7182fff871fa2a84596b3ea8be225495b1304c))
* migrate RegularLines to use context (18 props -&gt; 3 props) ([bbd4657](https://github.com/denolehov/annot/commit/bbd465786fbca43a4f913204bbf9045ade43a2d7))
* migrate SessionEditor to use context (11 props -&gt; 8 props) ([651099b](https://github.com/denolehov/annot/commit/651099bf5cf79f55a79a06453c7e16d5fb472236))
* migrate StatusBar to use context, wrap page with AnnotProvider ([12b94f6](https://github.com/denolehov/annot/commit/12b94f605780e2c84e9721620f5e9547eea71ce7))
* paste text into chip ([f9d2123](https://github.com/denolehov/annot/commit/f9d2123645a05f4a58b0a0ffc1ed66018445ead9))
* pre-populate empty bookmark labels with a derived label (user ([93f777c](https://github.com/denolehov/annot/commit/93f777c3f4f042cd83336c50665d69cfe2539091))
* proactive warning of mermaid syntax errors ([fab9dfe](https://github.com/denolehov/annot/commit/fab9dfe2d9ded358f619e1cabf694035040e1246))
* redesign bookmark command pallete edit page ([5814dcf](https://github.com/denolehov/annot/commit/5814dcf4dee25cde21d65107ed0b427be29c1b40))
* refine output format for LLMs ([5bbfc22](https://github.com/denolehov/annot/commit/5bbfc2236ed08afe3106f43da3758292cfa57723))
* reload fresh tags/bookmarks/exit modes when annot window ([2129fd3](https://github.com/denolehov/annot/commit/2129fd3cfe9c921601d753a04f87770441de3a86))
* remember window sizes and positions between runs ([ea1870b](https://github.com/denolehov/annot/commit/ea1870bf86e286e2301c3383ab4c6145461f9ef3))
* replace emojis in annotation editor with SVGs ([5a8c224](https://github.com/denolehov/annot/commit/5a8c2248e610c3d9676468ea12211065aa9291a7))
* rework /replace UX and visuals ([4b82f28](https://github.com/denolehov/annot/commit/4b82f28cd6ca7cea5bd7f70228df4fdb4d789c80))
* sealed editor outline is now white in the dark mode ([42d9060](https://github.com/denolehov/annot/commit/42d9060abab17fc51361c201a8cc345be2bb0760))
* set a nice icon for the app ([983deca](https://github.com/denolehov/annot/commit/983deca979538753b9b754cfe809d9290713bc72))
* set the different color on the codeblock circle depending on a language ([bb58d32](https://github.com/denolehov/annot/commit/bb58d3272d9436b85d5b48897262fd3104d2ae3e))
* set up release infra ([5843076](https://github.com/denolehov/annot/commit/5843076cfe5842fbb928bc6527b9524f8577abbb))
* simplify markdown heading breadcrumbs to just one (deepest) level ([006bc44](https://github.com/denolehov/annot/commit/006bc441b025cbb445322fad36c04a0cc204a542))
* tag, slash, and command palette filters now use fuzzy search ([e1255c8](https://github.com/denolehov/annot/commit/e1255c8f4708301aab846b8d241350aa97f74d1d))
* track tag usage ([14ff769](https://github.com/denolehov/annot/commit/14ff769f90bf27dfb88e250e53a8ba851aaa6832))
* unify copy code icons ([8a500bb](https://github.com/denolehov/annot/commit/8a500bb24095b1581d5053960ea24c7c6026a38b))
* update separator rendering ([0578534](https://github.com/denolehov/annot/commit/057853486a6e1cf6c1883654d431bf1d272d4952))
* use dot as a separator in bookmarks command palette instead of a circle ([9b655e4](https://github.com/denolehov/annot/commit/9b655e4ca23e77a595ae8c71ce7c2e72310bd56f))
* zoom out to fit the diagram if it wouldn't be smaller than 75% ([6ec1c0b](https://github.com/denolehov/annot/commit/6ec1c0b4fdba7bfdf2982765ea23f60a37c9fe7d))


### Bug Fixes

* /replace now positions cursor at the end of the inner replace ([2299950](https://github.com/denolehov/annot/commit/229995027d22bdfd3cde73fa54bfe589775d4dc4))
* action items in command palette are now executed when clicked ([1e36e32](https://github.com/denolehov/annot/commit/1e36e3267fa75142f267b231d65425853cc1eeb3))
* adjust dark mode colors for consistency ([c21c9b8](https://github.com/denolehov/annot/commit/c21c9b8767c360d84e01df6c1faafeeecd21170e))
* all popovers are now positioned correctly ([eeb82b7](https://github.com/denolehov/annot/commit/eeb82b75b5ae7d68d8ba6e0a8e17d65e29cbcdb6))
* allow cycling upwards from filtering mode in command palette ([3e4faec](https://github.com/denolehov/annot/commit/3e4faece8373c2370f16bb6031c77bee5d88191e))
* annotated portals have their line gutter highlighted correctly ([a9fa8f5](https://github.com/denolehov/annot/commit/a9fa8f5493b2a0204698c755d372b5dc13b34e3e))
* bookmark form is no longer clipping ([4395597](https://github.com/denolehov/annot/commit/4395597a77b111f5b050b39608b41ae05995eac1))
* bookmarks are now correctly syncronized across the app ([66ac4dc](https://github.com/denolehov/annot/commit/66ac4dcd596cce2e03f4f6db781d1725694ec67e))
* center the content when opening excalidraw diagrams ([584d94b](https://github.com/denolehov/annot/commit/584d94b8db1132fd71001932f59a51bc16bf5ef7))
* chips are now properly highlighted when caret is selecting them ([5b3fdf1](https://github.com/denolehov/annot/commit/5b3fdf197318d7a08705979b6cdb220e71272a82))
* chips are now properly vertically aligned ([03e4f85](https://github.com/denolehov/annot/commit/03e4f8521aaafe2295b6ee502c8de2fe3f311717))
* cmd-f highlights are now shown on tables, code blocks, and portals ([f213a65](https://github.com/denolehov/annot/commit/f213a65b2f7a05aa4753197791d9986d178737f6))
* command palette &gt; separator is now properly centered ([5847e6d](https://github.com/denolehov/annot/commit/5847e6d70abfb7982123c37b26657507af127efa))
* command palette is now closing on save if opened from editor ([001dd8c](https://github.com/denolehov/annot/commit/001dd8ccbbda028dcef878c01ed5f88c1787782a))
* command palette lists are now properly scrolling when using ([1c84cfb](https://github.com/denolehov/annot/commit/1c84cfb2e235016d43ae9dea48c9f4215eef1572))
* command pallete navigation UX improvements ([7bde5f2](https://github.com/denolehov/annot/commit/7bde5f29736e54b9dc3a72fdaf8d17122254641f))
* copy/save/export commands no longer fail in diff mode ([071cc3d](https://github.com/denolehov/annot/commit/071cc3d54920a2631c422236f97b92e5a8c56e5f))
* correctly render whitespace in a sealed annotation editor ([32c8304](https://github.com/denolehov/annot/commit/32c83040d6b1be8c12629f638662091a53227064))
* define a placeholder for the edit bookmark label field ([72f2746](https://github.com/denolehov/annot/commit/72f2746db5d5a39c488c1a39cfb5dd277572f02d))
* deleted, referenced bookmarks are no longer emitted as (deleted) in the output, but instead are displayed in full one last time ([e59ada1](https://github.com/denolehov/annot/commit/e59ada112d7703d59120d350ae960e9c25ff8b9f))
* deleting bookmarks no longer breaks UI ([1dfef53](https://github.com/denolehov/annot/commit/1dfef5346a78f81a1902d75f9b841b92002b4b41))
* deleting last item in no-create namespace automatically returns to namespace filter ([3a16031](https://github.com/denolehov/annot/commit/3a1603130283530f15c3268f3d7b845cf887f31f))
* disable devtools in prod builds ([7dd185c](https://github.com/denolehov/annot/commit/7dd185ca36212fcf83abae5017dee4e23a86658a))
* disarm pending delete on Esc instead of going back to previous ([cd645c2](https://github.com/denolehov/annot/commit/cd645c28c994dd2b92691489c75a6f0db0ef01dd))
* do not JPEGify on zoom ([ad9dc66](https://github.com/denolehov/annot/commit/ad9dc66ff81b76ac2943f7e0fd212dabe478ffa5))
* do not re-inject the bookmark context if it was created in the same session ([52d9821](https://github.com/denolehov/annot/commit/52d98218bdc5fcfa7e576e3521c3ae78fd5d363a))
* do not show &lt;tab&gt; next field if form has only one field ([c0a114a](https://github.com/denolehov/annot/commit/c0a114a043310050adbd731bd7b20b5560fddf04))
* ephemeral exit modes are no longer saved as permanent ([905d904](https://github.com/denolehov/annot/commit/905d904f0012f87d28f4a5d6463a251a89653264))
* excalidraw save button no longer triggers confirmation dialog ([bfd48ad](https://github.com/denolehov/annot/commit/bfd48ad875ae4578c12d9c15906e9c95530d4d36))
* focus on input element after deleting an item in command palette ([bffbb94](https://github.com/denolehov/annot/commit/bffbb94888c731167c46834d1e967c2cc62dec9f))
* footer no longer flashes when annot opens in dark mode ([098fd4e](https://github.com/denolehov/annot/commit/098fd4e6a96954b77094bfacbfd468136c64909c))
* highlights and hovers are no longer glitching ([cd0fcb2](https://github.com/denolehov/annot/commit/cd0fcb2190353ecf013d17a1dc15b72b02720e80))
* inline formatting in table cells is now rendered correctly ([5088d7c](https://github.com/denolehov/annot/commit/5088d7c42c027bca2576c764a91dcd155062e7e6))
* keep the cursor unchanged when hovering over bookmark chip ([71615c5](https://github.com/denolehov/annot/commit/71615c58f0a205d674b404d551c2855a58c18329))
* label-less bookmarks are now correctly showing their selection states in @ menu ([72c0534](https://github.com/denolehov/annot/commit/72c0534c1c36000983c949150c76cbbe1cc09f20))
* line breaks between paragraphs in sealed editors are now rendered ([961d97d](https://github.com/denolehov/annot/commit/961d97da0b62bec7fce00164da751f63bc4c85ef))
* line numbers in output on diff annotations are no longer lost ([f2ca7b9](https://github.com/denolehov/annot/commit/f2ca7b948c5f3d68b32bd1a67570dbaf8ad4a0c3))
* list bookmarks output now includes CREATED column ([d9698fa](https://github.com/denolehov/annot/commit/d9698fabbf29a312c41226c318bb2aefcc559817))
* local/global annotation and bookmark keybinds are now consistent ([938bdc6](https://github.com/denolehov/annot/commit/938bdc6a70aa9613021789bd960a9284a9f10e00))
* mermaid rendering broken when portals present ([81821e2](https://github.com/denolehov/annot/commit/81821e209f771e30a6a60182e57ae9841de3541f))
* plus hover button is now displayed over table fade thingy ([9cc5473](https://github.com/denolehov/annot/commit/9cc5473dfedf33b66e5555f2e063a8a49d462d11))
* pressing Esc when holding an active selection resets it ([2d03f83](https://github.com/denolehov/annot/commit/2d03f83460b9ebf5500fc8dbd601028edccdfa6e))
* prevent scrolling to random placed after shift-drag-release ([bd3d644](https://github.com/denolehov/annot/commit/bd3d644607cedab07b849ef64f22f8c2901b9e86))
* properly center mermaid diagrams ([d69e90e](https://github.com/denolehov/annot/commit/d69e90e557273d9447d699451f3604faf63ad46d))
* rendered mermaid diagrams are now correctly positioned relative to zoom controls ([2246447](https://github.com/denolehov/annot/commit/2246447d6d0b14e4bc4029add4e3e15cd2395e41))
* reword /replace tooltip ([9b2d53b](https://github.com/denolehov/annot/commit/9b2d53bd7dda9a26670ffb76675ebd9bb4fa4496))
* separators and punctuations are styled correctly now in syntax highlight ([1e4fda9](https://github.com/denolehov/annot/commit/1e4fda9faf7a8edb071c4c900fb4df07bb0523ca))
* show &lt;empty&gt; placeholder text when bookmark has no label and no ([730dd8a](https://github.com/denolehov/annot/commit/730dd8a4bc537ecebfc3843c0e45c5e5a2451d30))
* show toast on bookmark edit ([b574fd0](https://github.com/denolehov/annot/commit/b574fd0d34d74d8932d2db5a31a23ae8b270e93e))
* show toast on successful bookmark rename ([88e7ddf](https://github.com/denolehov/annot/commit/88e7ddf06d2c15ac28d24c7605606d931d490aa1))
* show warning and disable the "edit in excalidraw" button if mermaid ([4772054](https://github.com/denolehov/annot/commit/47720548ec1c2b82781caae7560849b3535dd57f))
* store display indices in annotation range for portal lookups ([d41704b](https://github.com/denolehov/annot/commit/d41704bef3f588b1367144b2994bea57dd0a43f1))
* syntax highlight in diff mode is now displaying diff added/removed ([a6df870](https://github.com/denolehov/annot/commit/a6df8701c34b09edae4e2a0cddc65f63667fce74))
* tag menu suggestion shrinks properly ([cbfc44f](https://github.com/denolehov/annot/commit/cbfc44f8871cff5ef18980b791e1d3d992e399d7))
* the code block annotation background no longer bleeds into the line gutter ([d89fb22](https://github.com/denolehov/annot/commit/d89fb22b7d1cf1a289795b3323bb20a118e70683))
* the entire bookmark context is referenced in the output ([c102758](https://github.com/denolehov/annot/commit/c1027587c50ae2ea5623c716f040bdc84cff8a06))
* the table row bookmark no longer breaks the layout ([a327f08](https://github.com/denolehov/annot/commit/a327f08ca8f1e082b045cdad0ef24169e8fdb3a2))
* toast in dark mode now uses correct background color ([1d5bf32](https://github.com/denolehov/annot/commit/1d5bf32f7d9339ad7c1e00cda443eddd6e1a003a))
* use a correct copy icon in the command palette ([3ba096c](https://github.com/denolehov/annot/commit/3ba096ce9710376fa773e060a85f0086110329c6))
* various visual and UX improvements for bookmarks ([21ce1f7](https://github.com/denolehov/annot/commit/21ce1f7d58fa8ae4be0aa8a229a7ce7c27947e74))
* warnings on exiting excalidraw modal appear consistently ([fdb5187](https://github.com/denolehov/annot/commit/fdb5187b1c874baa8937b4bb9138f6ff353eed63))
* zooming in and out no longer gets elements misaligned ([d0165ba](https://github.com/denolehov/annot/commit/d0165ba6877d5759c985170e07c5c6cbb5b9f02f))

## [0.1.0] - 2026-01-03

Initial release.
