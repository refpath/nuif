import { writeFile } from "node:fs/promises";

const fixedTime = Date.parse("2026-08-30T00:00:00Z");
const NativeDate = Date;
globalThis.Date = class extends NativeDate {
  constructor(...arguments_) {
    super(...(arguments_.length === 0 ? [fixedTime] : arguments_));
  }

  static now() {
    return fixedTime;
  }
};

const penpot = await import("@penpot/library");
const context = penpot.createBuildContext({ referer: "nuif-foreign-fixture" });
context.addFile({
  id: "00000000-0000-0000-0000-000000000001",
  name: "NUIF Penpot profile",
});
context.addPage({
  id: "00000000-0000-0000-0000-000000000011",
  name: "Profile page",
});
context.addBoard({
  id: "00000000-0000-0000-0000-000000000010",
  name: "Surface",
  x: 0,
  y: 0,
  width: 320,
  height: 200,
});
context.addRect({
  id: "00000000-0000-0000-0000-000000000021",
  name: "Card",
  x: 16,
  y: 20,
  width: 288,
  height: 160,
  fills: [{ fillColor: "#eff4ff", fillOpacity: 1 }],
});
context.addCircle({
  id: "00000000-0000-0000-0000-000000000022",
  name: "Status",
  x: 36,
  y: 48,
  width: 24,
  height: 24,
  fills: [{ fillColor: "#168b5b", fillOpacity: 1 }],
});
context.addText({
  id: "00000000-0000-0000-0000-000000000023",
  name: "Label",
  x: 76,
  y: 44,
  width: 180,
  height: 40,
  fills: [{ fillColor: "#1e293b", fillOpacity: 1 }],
  content: "NUIF Penpot profile",
  pluginData: {
    "org-nuif": {
      profile: "nuif-penpot-v3-0",
      font: "Ahem",
      "font-sha256": "f0a92cd0cc45735591c9b5b1fa8aecd5194e8dc518895ca22af94a46c23550dc",
      size: "16",
      "line-height": "24",
    },
  },
});
context.closeBoard();
context.closePage();
context.closeFile();

const bytes = await penpot.exportAsBytes(context);
await writeFile(new URL("fixture.penpot", import.meta.url), bytes);
process.stdout.write(`generated ${bytes.byteLength} bytes\n`);
