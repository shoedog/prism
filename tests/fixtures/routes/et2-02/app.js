const express = require('express');
const app = express();
const users = express.Router();
const teams = express.Router();
function listUsers(req, res) { res.end('users'); }
function listTeams(req, res) { res.end('teams'); }
users.get('/users', listUsers);
teams.get('/teams', listTeams);
app.use('/api', users, teams);
module.exports = app;
