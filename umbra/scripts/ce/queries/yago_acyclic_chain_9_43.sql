select count(*) from yago2 yago2_0, yago2 yago2_1, yago21, yago5 yago5_3, yago5 yago5_4, yago11, yago25, yago54, yago39 where yago2_0.s = yago2_1.s and yago2_1.d = yago21.d and yago21.s = yago5_3.s and yago5_3.d = yago5_4.d and yago5_4.s = yago11.s and yago11.d = yago25.d and yago25.s = yago54.d and yago54.s = yago39.s;