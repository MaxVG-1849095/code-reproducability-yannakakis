select count(*) from yago2_0, yago2_1, yago57, yago5, yago12, yago17 where yago2_0.s = yago2_1.s and yago2_1.d = yago57.d and yago57.s = yago5.d and yago5.s = yago12.s and yago12.d = yago17.d;