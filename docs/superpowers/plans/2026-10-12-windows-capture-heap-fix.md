# Windows 系统采集崩溃修复 — 2026-10-12

用户反馈：配对成功，但播放没有声音；手机随后报 Connection reset by peer (os104)，Windows 程序直接退出。

证据：Windows Application 1000/1001 多次记录 sonahop-windows.exe APPCRASH，异常码 0xc0000374（heap corruption）。读取已有 minidump 的异常线程 CONTEXT，用 DbgHelp 和原 Windows release PDB 解析栈上的返回地址，得到：

SystemCapture::new → Arc<ActivationStorage>::drop_slow → windows::PROPVARIANT::Drop → FreePropVariantArrayWorker → RtlFreeHeap → heap corruption。

源码核对：windows 0.62.2 的 PROPVARIANT Drop 实现在 src/extensions/Win32/System/StructuredStorage.rs，自动调用 PropVariantClear。ActivationStorage 的 _params 是 Rust Box，variant 的 VT_BLOB 指针借用该 Box。字段先释放 Box，之后外层 PROPVARIANT 自动释放同一 blob，造成二次释放/分配器所有权冲突。内部 union 的 ManuallyDrop 并不能禁止外层 PROPVARIANT 的析构。

修复：外层 variant 改为 ManuallyDrop<PROPVARIANT>，API 指针显式借用其内部 PROPVARIANT；Box 是唯一释放参数内存的所有者，Arc 仍保证异步激活完成/超时后回调所需的生命周期。

仅构建 Windows，保留 Android foreground-fix APK 和协议。未启动应用、录制/播放或执行自动测试；用户复验开始共享后不再退出，再检查发包与手机播放。
构建结果：Windows release 成功。sonahop-windows-heap-fix.exe，11258368 bytes，SHA256 0F608ABC180380D3008E5E74CE67CBC5C27D22A1074491EF71A2340FA07C4E25。日志 artifacts/audio-mvp/windows-heap-fix-build.log。
