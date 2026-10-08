import clsx from "clsx";
import React, { ReactNode } from "react";
import { Link } from "wouter";

import {
  workbench,
  workbench__content,
  workbench__header,
  workbench__header__link,
  workbench__header__linkActive,
  workbench__header__track,
} from "./WorkbenchLayout.module.css";

function activeClassName(isActive: boolean) {
  return clsx(workbench__header__link, {
    [workbench__header__linkActive]: isActive,
  });
}

export function WorkbenchLayout({ children }: { children: ReactNode }) {
  return (
    <div className={workbench}>
      <div className={workbench__header}>
        <div className={workbench__header__track}>
          <Link className={workbench__header__link} href="/">
            Paddler 🏓
          </Link>
        </div>
        <div className={workbench__header__track}>
          <Link className={activeClassName} href="/">
            Dashboard
          </Link>
          <Link className={activeClassName} href="/model">
            Model
          </Link>
          <Link className={activeClassName} href="/prompt">
            Prompt
          </Link>
        </div>
      </div>
      <div className={workbench__content}>{children}</div>
    </div>
  );
}
