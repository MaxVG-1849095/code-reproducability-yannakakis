select count(*) from yago2 yago2_0, yago2 yago2_1, yago2 yago2_2, yago2 yago2_3, yago17 yago17_4, yago5 yago5_5, yago5 yago5_6, yago17 yago17_7, yago21 yago21_8, yago21 yago21_9, yago58, yago5 yago5_11 where yago2_0.s = yago2_1.s and yago2_1.d = yago17_4.s and yago2_2.s = yago2_3.s and yago2_2.d = yago58.d and yago17_4.d = yago5_5.d and yago5_5.s = yago5_6.s and yago5_6.d = yago17_7.s and yago17_7.d = yago21_8.d and yago21_8.s = yago21_9.s and yago21_9.d = yago5_11.d and yago58.s = yago5_11.s;