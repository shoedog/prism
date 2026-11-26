import quay from './quay';
export const read = (vote: 'up' | 'down') => quay.get(`/vote/${vote}`); // GET /vote/{}
