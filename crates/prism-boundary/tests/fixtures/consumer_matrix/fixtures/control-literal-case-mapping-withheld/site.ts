import quay from './quay';
export const read = () => quay.get('/user/ME'); // GET /user/ME
