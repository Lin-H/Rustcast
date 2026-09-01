import { render } from "preact";
import { App } from "./App";
import "./index.css";

const root = document.getElementById("root");

if (root === null) {
  throw new Error("Rustcast UI root element not found");
}

render(<App />, root);
