const express = require('express');
const app = express();
const staff = express.Router();

function team(req, res) { res.send('team'); }
function invite(req, res) { res.send('invited'); }

staff.get('/team', team).post('/invite', invite);
app.use('/staff', staff);

module.exports = app;
