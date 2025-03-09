import { FC } from 'react'

const AppBar: FC = () => {
  return (
    <header className="flex flex-row items-center pl-20 h-10 border-b bg-zinc-500 dark:bg-zinc-500 transition-colors duration-300 ease-in-out">
      <div className="appbar w-full h-full"></div>
      <div>Zelan</div>
    </header>
  )
}

export default AppBar
