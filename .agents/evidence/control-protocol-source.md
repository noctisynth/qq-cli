# 固定 WebUI 登录观察协议核对

文档编号：CHECK-016。状态：固定源码/发布包观察，非真实协议验收。更新：2026-10-18。
关联：CHECK-015、DATA-001、FACT-002、RFC-0003、SPEC-004、TASK-002。

## 基线与来源

NapCat v4.18.38，提交bbd0d2aca3d470c20b33b1d4f95123335633bf26，检索2026-10-18。只读Git对象，缺失对象从原公开仓库取得；研究副本仅在忽略目录。发布包napcat.mjs摘要仍为DATA-001的f79260cb7e6c4b579a4346ffd39a91a33ea73f9b7cd44d9ce204ab471096601a，包含CheckLoginStatus、GetQQLoginQrcode、generatePasswordHash、require2FA、qrLoginAccepted等符号；符号出现不等于完整运行验证。

以下文件均位于固定提交的packages/napcat-webui-backend；[固定源码目录](https://github.com/NapNeko/NapCatQQ/tree/bbd0d2aca3d470c20b33b1d4f95123335633bf26/packages/napcat-webui-backend)。

| 文件 | Git blob | 观察 |
|---|---|---|
| src/api/Auth.ts | 05aafd79b0d2baa5effacb48cf885492435bc854 | hash鉴权，可能返回require2FA；成功返回Credential，不是QQ登录 |
| src/helper/SignToken.ts | 39e048a6c862dbaad5b30710187f6ac9d2a38db4 | SHA-256(token + .napcat)，服务端签发base64 JSON凭据；有效期1小时，客户端不自行签发 |
| src/middleware/auth.ts | 03ffa66bbe16ce2acd92f03cbf571ab356e28e1e | /auth/login豁免；其余接口校验Bearer凭据，亦支持query但CLI不使用 |
| src/router/index.ts | 853e48f5e89fa8a6b355793c1c17f7bdc83c78b1 | 鉴权中间件后挂auth和QQLogin路由；外层/api挂载见CHECK-015的WebUI入口 |
| src/router/auth.ts | f36bca614d2cfb851d61b321cd3a3e10e938036b | POST /auth/login |
| src/router/QQLogin.ts | 101aa50daea110b3788f25bc183c6f4e6172f4b3 | POST /QQLogin/CheckLoginStatus；另有快速/密码/刷新/设备/重启接口，各有副作用 |
| src/api/QQLogin.ts | 07d007f99d12d4c05bf2053ccd2c0903b0cdf29c | isLogin/coreReady均要求QQ标记及online明确true，isOffline单独表示已登录但离线；返回二维码URL/错误正文，CLI不得透传 |
| src/helper/Data.ts | 697211ce3c34e7d54e8e8c9bcbaea0bb78d1a21c | 默认waiting_qrcode，刷新过程会改登录阶段；读取阶段不等于实际账号证明 |
| src/types/index.ts | 43c2e5619bd38a905d99973465c827467178a4bc | 七种固定阶段：waiting_qrcode/generating_qrcode/qrcode_scanned/initializing/ready/offline/reconnecting |
| src/utils/response.ts | c97d3d974501a0cbdb133a5d3b5de46404b53cbb | HTTP 200封装code/data/message，错误也可能200，不能仅凭HTTP成功 |
| src/const/status.ts | c65e93867728696f830d0f8da39fb75fb84ef218 | Success=0、Error=-1 |

## 接入边界

候选只调用固定POST /api/auth/login及/api/QQLogin/CheckLoginStatus，独立管理token和Credential仅在内存，不接收QQ密码、不自动选择快速账号、不刷新二维码、不重置设备或重启。鉴权会触及WebUI本地登录频率/凭据管理，不声称它完全无副作用；不发起QQ账号登录。2FA要求、拒绝或异常不自动重试/降级。

调用前仍必须恢复原签名预登录进程/Job/固定端点，向同一受检socket发请求；真实兼容仍受既有门禁。观察只映射固定阶段和布尔提示，未知阶段统一unknown，不回显原始阶段、二维码URL、错误正文、token、hash或凭据；控制ready只能提示再核对OneBot身份，不能提升登记ready或绑定账号。

本证据不证明控制接口已接入CLI、真实QQ提供同进程监听、登录恢复成功或焦点共存；具体实现验收另见CHECK-013。
