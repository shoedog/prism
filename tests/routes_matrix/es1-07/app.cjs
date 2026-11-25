const express = require('express');
const routers = require('./routers.cjs');
const { archive: archiveRouter } = require('./routers.cjs');
const app = express();
app.use('/archive', archiveRouter);
app.use('/audit', routers.audit);
module.exports = app;
