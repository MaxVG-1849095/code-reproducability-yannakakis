select count(*) from imdb100, imdb126, imdb14, imdb38 where imdb100.d = imdb126.d and imdb126.d = imdb14.s and imdb14.s = imdb38.s;