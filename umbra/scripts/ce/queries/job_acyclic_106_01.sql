select count(*) from imdb1, imdb30, imdb117, imdb103 where imdb1.s = imdb30.s and imdb30.s = imdb117.s and imdb117.d = imdb103.s;