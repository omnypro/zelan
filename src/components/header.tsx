import { FC } from "react"

export const Header: FC = () => {
  return (
    <header
      className="sticky top-0 z-50 flex items-center w-full p-1.5"
      data-tauri-drag-region
    >
      <div
        className="flex items-center w-full gap-2 px-4"
        data-tauri-drag-region
      >
        <span>Zelan</span>
      </div>
    </header>
  );
}
