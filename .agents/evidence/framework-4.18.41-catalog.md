# Framework 4.18.41 固定资产

文档编号：CHECK-111。日期：2026-10-24。关联：CHECK-072、TASK-006。起点：8b630e7。

安装catalog新增v4.18.41（接受带或不带v的精确版本）；不改默认版本，不接受latest。来源为 https://github.com/NapNeko/NapCatQQ/releases/download/v4.18.41/NapCat.Framework.zip ，提交859eb530c5801577633c2c3ae871d23586edf281，继承CHECK-072固定来源记录；本次未重新查询线上发布状态。

本地留存原始ZIP只读重算：36,738,019字节，SHA-256 3c526f5789ff6cc56d78381eb10536c8273fe394d227187b27dea94b59965822，与CHECK-072及原资产核验记录一致。通过实际archive::prepare生成临时展开清单：680文件、118,959,189展开字节、11项关键摘要全部匹配，临时树由Prepared释放。探针及上游资产未提交，也未启动QQ或读取账号。

保留七个既有Windows/公共入口成员，新增package.json及napi2native、packet、ffmpeg的darwin.arm64成员摘要；实际摘要值以catalog/pins.rs为单一实现位置。完整ZIP摘要覆盖所有成员，不因关键成员列表省略其它内容认证。旧版本摘要与默认选择保持原值。许可注记仍为未确认，资产身份不是许可或兼容性结论。

QQ官网DMG来源认证、固定角色映射、部署输入生产者及正式启动尚待接合；本次没有把App Store安装与官网DMG视为相同字节基线。Windows原生门禁未运行。

验证通过：scripts/check-macos.sh（22项Node、fmt、Clippy、340项Rust通过/2项忽略、arm64 release）；源码包313项覆盖全部源码且排除内部/私人材料。受影响上下文相对链接、私人路径检查与git diff --check通过。Semifold变更集framework-4-18-41-catalog，status通过；仅本地提交，不推送/发布。
