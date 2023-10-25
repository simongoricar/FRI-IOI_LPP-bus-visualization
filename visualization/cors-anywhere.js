// Listen on a specific host via the HOST environment variable
const host = process.env["HOST"] || "0.0.0.0";
// Listen on a specific port via the PORT environment variable
const port = process.env["PORT"] || 8855;

import { createServer } from "cors-anywhere";

createServer({
  originWhitelist: ["http://localhost:5173"],
  requireHeader: ['origin', 'x-requested-with'],
  removeHeaders: ['cookie', 'cookie2']
}).listen(port, host, function() {
  console.log('Running CORS Anywhere on ' + host + ':' + port);
});
