select count(*) from yago0, yago2_1, yago2_2, yago11, yago5, yago2_5 where yago0.s = yago2_1.d and yago2_1.d = yago2_5.d and yago2_1.s = yago2_2.s and yago2_2.d = yago11.s and yago11.s = yago5.d;