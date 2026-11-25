import express from 'express';
export const guides = express.Router();
function guide(req, res) { res.send('guide'); }
guides.get('/guide', guide);
