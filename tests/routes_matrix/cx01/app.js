const express = require('express');
const { auditRouter } = require('./routers');

const app = express();
app.use('/internal', auditRouter);

module.exports = app;
