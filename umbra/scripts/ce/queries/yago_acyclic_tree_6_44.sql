select count(*) from yago5 yago5_0, yago2 yago2_1, yago2 yago2_2, yago5 yago5_3, yago5 yago5_4, yago5 yago5_5 where yago5_0.d = yago2_1.d and yago2_1.d = yago5_3.d and yago5_3.d = yago5_4.d and yago5_4.d = yago5_5.d and yago2_1.s = yago2_2.s;