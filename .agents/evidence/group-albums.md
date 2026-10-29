# 群相册查询上传与显式互动

文档编号：CHECK-156。状态：R14九action协议与七领域命令离线通过。日期：2026-10-30。
关联：RFC-0009、SPEC-009、TASK-007、CHECK-154、CHECK-155。

固定NapCat v4.18.38/bbd0d2aca3d470c20b33b1d4f95123335633bf26的八个extends相册处理器、core/apis/webapi.ts、NodeIKernelAlbumService.ts、data/album.ts已核对。上传单/多图片与视频均await本地读取/分片；多图共批次，视频先主体后封面，提帧失败用默认封面；SDK删除/评论/点赞返回unknown，明确非零数字result失败不折成接受。源码/示例/推断的返回形状边界按SPEC-009保留，不引用真实材料。

album list/media显式群和原生续页参数，一次读取有界页；本地截断不提供会跳过未返回项的续页信息，complete:false。列表处理器丢原生状态，name优先SDK字段；媒体只按处理器示例media_id/url投影，无已知数组则ALBUM_SCHEMA_UNSUPPORTED；不猜lloc、不签发m1/f1、不访问返回URL。真实媒体返回覆盖仍待另验。

remove、like显式true/false及可选lloc、comment分别构造严格载荷；无lloc为批次操作。upload支持1至32个普通非空本地图片，一图/多图分别使用对应action；upload-video用显式本地文件。复用128MiB单文件、1GiB/256条缓存和macOS运行前缀；领域只向上游发送受管影本路径，不以base64截断大文件或传任意调用方路径。九项效应、group_id目标与Rust实际载荷Schema完成；所有业务受read/send/群targets策略约束。

共享文件计划仅按已审阅action固定file/files及持久性，稳定单/多内容指纹与私有fileBinding替代随机路径。全部影本在持久pending和实际账号核对后认证保留、原生请求前可读取；同键缓存不再保留/调用，不同内容冲突。保持既有emoji单文件指纹和persistent语义；相册await读取终止的影本沿正常保留期清理，pending/unknown不清理。多步上传可能部分完成，不制造媒体ID或取消/交付回执。

原创受管CLI17分支含列表/截断/媒体/原生失败/未知形状、删/赞/取消/评、单图/批量/视频、批量后切号与三种策略拒绝；均合成文件和loopback进程。验证pending先于原生访问、内容一致/意图认证、缓存一次、改内容冲突、终态过期清理和unknown保留，ledger无源名/正文/文件内容。预览与必填/32文件上限、实际结果Schema通过。API单测12、album2、emoji_add2、附件4、缓存3、API3/API CLI1、CLI14、Skill3及social2回归通过。

all-targets Clippy、fmt、架构/契约、Skill quick_validate、源码包白名单499文件、diff通过；115叶命令/132帮助入口，10任务/13导出文件；Semifold仅规划19份变更集至0.2.0，未消费/发布。未重跑完整suite/release，旧完整验证仍按CHECK-152快照。真实相册媒体形状/分页、图片/视频解码、原生未知状态、上传效果及macOS未验收；本次无真实QQ操作或push。
