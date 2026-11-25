import express from 'express';
export const shipment = express.Router();
export const inventory = express.Router({ mergeParams: true });
function orders(req, res) { res.send('orders'); }
function stock(req, res) { res.send('stock'); }
shipment.get('/orders', orders);
inventory.get('/stock', stock);
