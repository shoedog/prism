const express = require('express');
const app = express();

function schedule(req, res) { res.send('schedule'); }

app.route('/reserved');
app.route('/schedule').get(schedule);

module.exports = app;
