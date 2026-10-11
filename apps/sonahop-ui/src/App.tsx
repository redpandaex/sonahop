import { DesktopScreen } from "./screens/desktop";
import { MobileScreen } from "./screens/mobile";
import { useWorkbench } from "./lib/use-workbench";
import "./App.css";

export default function App() {
  const workbench = useWorkbench();
  return (
    <>
      {workbench.info.browserPreview && (
        <div className="browser-preview-tools">
          <span>布局预览</span>
          <select
            aria-label="预览平台"
            value={workbench.info.platform}
            onChange={(event) => {
              const url = new URL(location.href);
              url.searchParams.set("platform", event.currentTarget.value);
              location.assign(url);
            }}
          >
            <option value="windows">Windows</option>
            <option value="android">Android</option>
          </select>
        </div>
      )}
      {workbench.info.platform === "android" ? (
        <MobileScreen workbench={workbench} />
      ) : (
        <DesktopScreen workbench={workbench} />
      )}
    </>
  );
}
