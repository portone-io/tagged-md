import { md } from "tagged-md";

const app = document.getElementById("app") as HTMLElement;

app.innerHTML = md`
# Hello, world!

- [x] GFM supported
`;
