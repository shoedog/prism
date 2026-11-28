function fa(olda) { return olda; }
(function (root) { root.fa = function (ra) { return ra; }; })(this);
function runfa(x1) { return fa(x1); }
