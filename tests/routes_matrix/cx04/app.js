const express = require('express');

const app = express();
function tagRequest(req, res, next) {
    res.set('X-Area', 'billing');
    next();
}

app.use('/billing', tagRequest);

module.exports = app;
