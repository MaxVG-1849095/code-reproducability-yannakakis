select count(*) from yago2 yago2_0, yago2 yago2_1, yago2 yago2_2, yago2 yago2_3, yago21 yago21_4, yago5 yago5_5, yago5 yago5_6, yago21 yago21_7, yago2 yago2_8, yago2 yago2_9, yago2 yago2_10, yago2 yago2_11 where yago2_0.s = yago2_1.s and yago2_1.d = yago21_4.d and yago2_2.s = yago2_3.s and yago2_2.d = yago2_10.d and yago21_4.s = yago5_5.s and yago5_5.d = yago5_6.s and yago5_6.d = yago21_7.d and yago21_7.s = yago2_8.d and yago2_8.s = yago2_9.s and yago2_9.d = yago2_11.d and yago2_10.s = yago2_11.s;