select count(*) from yago2 yago2_0, yago2 yago2_1, yago35 yago35_2, yago5, yago35 yago35_4, yago2 yago2_5, yago2 yago2_6, yago17 yago17_7, yago17 yago17_8, yago46, yago17 yago17_10, yago17 yago17_11 where yago2_0.s = yago2_1.s and yago2_1.d = yago2_5.d and yago35_2.s = yago5.s and yago35_2.d = yago35_4.d and yago5.d = yago17_11.d and yago2_5.s = yago2_6.s and yago2_6.d = yago17_7.d and yago17_7.s = yago17_8.s and yago17_8.d = yago46.d and yago46.s = yago17_10.d and yago17_10.s = yago17_11.s;