# F4 history replay

Pin: `6744295fa888e0621fc9f9a8ace04f1a4daf8554`
Inventory recorded before consumer extraction: 2026-11-25T03:38:54.572423+00:00
First-parent commits examined: 607
Commits with element changes before reverts: 43
Usable commits: 37
Operation-only commits: 5
Field-only commits: 26
Commits with both: 6
Commits with operation changes including overlap: 11
Commits with field changes including overlap: 32
Usable element changes by kind:
- enum_values_changed: 12
- field_removed: 218
- field_required_added: 15
- field_type_changed: 20
- op_removed: 18

Replay wall time: 74.011 s (parent materialization through replay, excluding inventory and reader packet)
Replay completed: 2026-11-25T03:40:10.558831+00:00
Replayed usable commits: 37
Skipped config_missing: 0
Skipped no_client_import: 0
Element occurrences with predictions: 194
Predicted locations: 194
Control locations: 616
Predicted outcomes: {'later': 70, 'present_at_pin': 91, 'same_commit': 33}
R3 packet: 150 selected of 161 eligible, 11 outside the cap

Lift uses touches in first-parent successors +1 through +10 after C.
Pooled lift: 52/194 vs 5/616, lift 33.023

## Per-commit lift

- `0fe709d84b7ae2a7e7ef0f7c18be49de5c6d9bc7` 0/0 vs 1/5, lift undefined
- `7c8c1cabcb3a1f66f3973b777bc74659bcd625dd` 0/0 vs 1/5, lift undefined
- `e2adc9b88703023de07afbc16f3f4db62a7f6f0f` 0/2 vs 0/6, lift undefined
- `69c6b22b418b6420c71c4d3efec07785449c8910` 0/2 vs 0/6, lift undefined
- `ed41d4a2f51be19b9b86cea89029c13ebfb250d4` 0/0 vs 0/5, lift undefined
- `5a2fbeccc274dad85c381f38bbf73b438173a3f8` 0/0 vs 0/5, lift undefined
- `07d3d3525ba5c2248bafa7ea6812d811dff476b3` 0/2 vs 0/6, lift undefined
- `e17501311b8b3909921d81bacbaae2a728e29080` 0/0 vs 0/5, lift undefined
- `5e89f4031b3c9357698dee450205f973e16243cb` 0/1 vs 0/5, lift undefined
- `a613315f4fc84a85a2abc90813ff678fb46e7d41` 0/1 vs 0/5, lift undefined
- `2b1ad94791631c35ebe684d6b3b5e05271859d76` 0/6 vs 0/18, lift undefined
- `cfcf4ba02f3bb8770a0ea9fccc95b4b91b34432a` 0/0 vs 0/5, lift undefined
- `661d8a7f8fec684a4b07fe5f4b67a53642fdf4d3` 0/6 vs 2/18, lift 0.000
- `a6c51b4f063181e058e01787d6b20c1b00aec16d` 0/0 vs 0/5, lift undefined
- `b64b5e61b4356eeb4526d43ac73b96fe2a110f58` 0/5 vs 0/15, lift undefined
- `7085cfa7ed54c1ce397d0a1d605fa6f3442838c9` 0/2 vs 0/6, lift undefined
- `012bcdc071ebf7301cd84dbd3923cea73a3cf039` 0/0 vs 0/5, lift undefined
- `f93fb6b7093d6b70628a36ee489bd6e5d7d9710e` 0/14 vs 0/42, lift undefined
- `5543d0adbbc427d0428f8658946b349fc0630e14` 0/0 vs 0/5, lift undefined
- `da4be341f2f7b89a1404a1080dc39bd27e693dbf` 0/1 vs 0/5, lift undefined
- `1dba13a278a0a56ce2e0fd82b21ebd35ef73d538` 0/7 vs 0/21, lift undefined
- `8d421ae5d5ac24171053493414a7fb247d4ba4f3` 0/4 vs 0/12, lift undefined
- `37f0ffcffacb1204fb59a94b6c12e253f7daad62` 0/0 vs 0/5, lift undefined
- `785b05f31e18ac0576fd0e19d7ee3c2513bdc770` 0/0 vs 0/5, lift undefined
- `61b10a854ff6ea5bf5eedbc81d4a373aecbeac9e` 0/3 vs 0/9, lift undefined
- `ff61e91fe662ef0d39f5ea8488b7f00f8fc1165c` 0/1 vs 0/5, lift undefined
- `7a036f2118e25292a2cae1d9f2b6ec8a6f31d6de` 0/1 vs 0/5, lift undefined
- `fd0b62c1f1cb9bd7c95a54d6f8bfcf2689b00275` 0/24 vs 0/72, lift undefined
- `82fdfc77636d8d1ce28710d929a8c22bb52834ef` 0/3 vs 0/9, lift undefined
- `530812f81097cbab0c2b42b163539bf03b5aa54f` 0/37 vs 0/109, lift undefined
- `f723d120d924dcf7e65b969ce18f8111cac40343` 0/1 vs 0/5, lift undefined
- `f05f1eb80d10124c252b1b3f812aa26e95aad8f8` 2/2 vs 0/6, lift undefined
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` 40/54 vs 1/129, lift 95.556
- `c2a0bee7dc5e6066ab20c6417fe9f89f6c553a86` 2/4 vs 0/12, lift undefined
- `3264fdd1d90f8845dac1632b772bc86d2d61a554` 6/8 vs 0/24, lift undefined
- `73cfbace7090b233a51571dabdd26ec47c62c890` 2/2 vs 0/6, lift undefined
- `3b6f981b81ca4376138734e4039eab59b1fea63c` 0/1 vs 0/5, lift undefined

## Late touches

- `e2adc9b88703023de07afbc16f3f4db62a7f6f0f` → `61b10a854ff6ea5bf5eedbc81d4a373aecbeac9e` `field:resp:GET /answer/api/v1/search:200#/data/list/[]/object/user_info/status` at `ui/src/services/client/search.ts:10` (+304 commits)
- `a613315f4fc84a85a2abc90813ff678fb46e7d41` → `8d421ae5d5ac24171053493414a7fb247d4ba4f3` `field:resp:GET /answer/api/v1/user/info:200#/data/avatar` at `ui/src/services/common.ts:119` (+141 commits)
- `661d8a7f8fec684a4b07fe5f4b67a53642fdf4d3` → `5f113285a1db06483292d7affc35a305fca7bd9c` `field:req:POST /answer/api/v1/user/login/email#/e_mail` at `ui/src/services/common.ts:85` (+102 commits)
- `661d8a7f8fec684a4b07fe5f4b67a53642fdf4d3` → `8d421ae5d5ac24171053493414a7fb247d4ba4f3` `field:req:POST /answer/api/v1/user/login/email#/pass` at `ui/src/services/common.ts:85` (+108 commits)
- `f93fb6b7093d6b70628a36ee489bd6e5d7d9710e` → `c602752dcce640117c981e4d43fdf1ba9a3fef0b` `field:req:PUT /answer/api/v1/tag/synonym#/synonym_tag_list/[]/parsed_text` at `ui/src/services/client/tag.ts:25` (+102 commits)
- `f93fb6b7093d6b70628a36ee489bd6e5d7d9710e` → `da4be341f2f7b89a1404a1080dc39bd27e693dbf` `field:req:PUT /answer/api/v1/comment#/parsed_text` at `ui/src/services/common.ts:60` (+27 commits)
- `f93fb6b7093d6b70628a36ee489bd6e5d7d9710e` → `da4be341f2f7b89a1404a1080dc39bd27e693dbf` `field:req:POST /answer/api/v1/comment#/parsed_text` at `ui/src/services/common.ts:70` (+27 commits)
- `f93fb6b7093d6b70628a36ee489bd6e5d7d9710e` → `2d02452e172068268fd5099ee88c980096d67561` `field:req:PUT /answer/api/v1/question#/tags/[]/parsed_text` at `ui/src/services/common.ts:87` (+362 commits)
- `f93fb6b7093d6b70628a36ee489bd6e5d7d9710e` → `61b10a854ff6ea5bf5eedbc81d4a373aecbeac9e` `field:req:PUT /answer/api/v1/answer#/html` at `ui/src/services/common.ts:91` (+145 commits)
- `f93fb6b7093d6b70628a36ee489bd6e5d7d9710e` → `2d02452e172068268fd5099ee88c980096d67561` `field:req:POST /answer/api/v1/question#/tags/[]/parsed_text` at `ui/src/services/common.ts:168` (+362 commits)
- `1dba13a278a0a56ce2e0fd82b21ebd35ef73d538` → `d93e31e92afa500aec40090f2f15d0c0cf3c5639` `field:resp:POST /answer/api/v1/user/register/email:200#/data/is_admin` at `ui/src/services/common.ts:102` (+379 commits)
- `1dba13a278a0a56ce2e0fd82b21ebd35ef73d538` → `61b10a854ff6ea5bf5eedbc81d4a373aecbeac9e` `field:resp:GET /answer/api/v1/user/register/captcha:200#/data/is_admin` at `ui/src/services/common.ts:107` (+111 commits)
- `8d421ae5d5ac24171053493414a7fb247d4ba4f3` → `61b10a854ff6ea5bf5eedbc81d4a373aecbeac9e` `field:req:PUT /answer/api/v1/user/password#/pass` at `ui/src/services/common.ts:138` (+56 commits)
- `61b10a854ff6ea5bf5eedbc81d4a373aecbeac9e` → `530812f81097cbab0c2b42b163539bf03b5aa54f` `field:resp:GET /answer/admin/api/setting/privileges:200#/data/options/[]/level` at `ui/src/services/admin/settings.ts:164` (+163 commits)
- `61b10a854ff6ea5bf5eedbc81d4a373aecbeac9e` → `530812f81097cbab0c2b42b163539bf03b5aa54f` `field:resp:GET /answer/admin/api/setting/privileges:200#/data/selected_level` at `ui/src/services/admin/settings.ts:164` (+163 commits)
- `61b10a854ff6ea5bf5eedbc81d4a373aecbeac9e` → `530812f81097cbab0c2b42b163539bf03b5aa54f` `field:req:PUT /answer/admin/api/setting/privileges#/level` at `ui/src/services/admin/settings.ts:170` (+163 commits)
- `fd0b62c1f1cb9bd7c95a54d6f8bfcf2689b00275` → `d93e31e92afa500aec40090f2f15d0c0cf3c5639` `field:resp:POST /answer/api/v1/user/register/email:200#/data/ip_info` at `ui/src/services/common.ts:105` (+189 commits)
- `fd0b62c1f1cb9bd7c95a54d6f8bfcf2689b00275` → `439e7862946ed8cc744389f2b4bd6daf6dfd14b7` `field:req:POST /answer/api/v1/answer/acceptance#/question_id` at `ui/src/services/common.ts:209` (+88 commits)
- `f05f1eb80d10124c252b1b3f812aa26e95aad8f8` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `op:GET /answer/admin/api/siteinfo/write` at `ui/src/services/admin/settings.ts:126` (+10 commits)
- `f05f1eb80d10124c252b1b3f812aa26e95aad8f8` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `op:PUT /answer/admin/api/siteinfo/write` at `ui/src/services/admin/settings.ts:130` (+10 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/code` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/authorized_attachment_extensions` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/authorized_image_extensions` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/max_attachment_size` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/max_image_megapixel` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/max_image_size` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/min_content` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/min_tags` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/recommend_tags` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/recommend_tags/[]/display_name` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/recommend_tags/[]/slug_name` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/required_tag` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/reserved_tags` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/reserved_tags/[]/display_name` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/reserved_tags/[]/slug_name` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/data/restrict_answer` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/msg` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/write:200#/reason` at `ui/src/services/admin/settings.ts:126` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/authorized_attachment_extensions` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/authorized_image_extensions` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/max_attachment_size` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/max_image_megapixel` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/max_image_size` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/min_content` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/min_tags` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/recommend_tags` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/recommend_tags/[]/display_name` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/recommend_tags/[]/slug_name` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/required_tag` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/reserved_tags` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/reserved_tags/[]/display_name` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/reserved_tags/[]/slug_name` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/write#/restrict_answer` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:PUT /answer/admin/api/siteinfo/write:200#/code` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:PUT /answer/admin/api/siteinfo/write:200#/data` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:PUT /answer/admin/api/siteinfo/write:200#/msg` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:PUT /answer/admin/api/siteinfo/write:200#/reason` at `ui/src/services/admin/settings.ts:130` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/api/v1/siteinfo:200#/data/site_write` at `ui/src/services/common.ts:317` (+9 commits)
- `9efa9471bd841ddcad2f5e97bf5def12ad96756e` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/api/v1/siteinfo:200#/data/site_write/restrict_answer` at `ui/src/services/common.ts:317` (+9 commits)
- `c2a0bee7dc5e6066ab20c6417fe9f89f6c553a86` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/interface:200#/data/default_avatar` at `ui/src/services/admin/settings.ts:78` (+8 commits)
- `c2a0bee7dc5e6066ab20c6417fe9f89f6c553a86` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/interface:200#/data/gravatar_base_url` at `ui/src/services/admin/settings.ts:78` (+8 commits)
- `3264fdd1d90f8845dac1632b772bc86d2d61a554` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/general:200#/data/check_update` at `ui/src/services/admin/settings.ts:59` (+7 commits)
- `3264fdd1d90f8845dac1632b772bc86d2d61a554` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/general#/check_update` at `ui/src/services/admin/settings.ts:73` (+7 commits)
- `3264fdd1d90f8845dac1632b772bc86d2d61a554` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `op:PUT /answer/admin/api/siteinfo/legal` at `ui/src/services/admin/settings.ts:140` (+7 commits)
- `3264fdd1d90f8845dac1632b772bc86d2d61a554` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/admin/api/siteinfo/login:200#/data/login_required` at `ui/src/services/admin/settings.ts:172` (+7 commits)
- `3264fdd1d90f8845dac1632b772bc86d2d61a554` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/login#/login_required` at `ui/src/services/admin/settings.ts:178` (+7 commits)
- `3264fdd1d90f8845dac1632b772bc86d2d61a554` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:resp:GET /answer/api/v1/siteinfo:200#/data/login/login_required` at `ui/src/services/common.ts:317` (+7 commits)
- `73cfbace7090b233a51571dabdd26ec47c62c890` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/interface#/default_avatar` at `ui/src/services/admin/settings.ts:91` (+3 commits)
- `73cfbace7090b233a51571dabdd26ec47c62c890` → `630ac20a384cad4b4e1527528c2fbbc50b937933` `field:req:PUT /answer/admin/api/siteinfo/interface#/gravatar_base_url` at `ui/src/services/admin/settings.ts:91` (+3 commits)
