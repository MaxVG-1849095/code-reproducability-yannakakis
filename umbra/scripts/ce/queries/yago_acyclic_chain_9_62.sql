select count(*) from yago0 yago0_0, yago0 yago0_1, yago0 yago0_2, yago0 yago0_3, yago2 yago2_4, yago6 yago6_5, yago6 yago6_6, yago2 yago2_7, yago2 yago2_8 where yago0_0.d = yago0_1.d and yago0_1.s = yago0_2.s and yago0_2.d = yago0_3.d and yago0_3.s = yago2_4.d and yago2_4.s = yago6_5.d and yago6_5.s = yago6_6.s and yago6_6.d = yago2_7.s and yago2_7.d = yago2_8.d;